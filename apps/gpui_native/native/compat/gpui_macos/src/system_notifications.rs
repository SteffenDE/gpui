//! System notifications through the `UNUserNotificationCenter` API.
//!
//! Authorization reads never prompt; explicit requests may show the system dialog.
//! Every entry point guards unbundled processes before accessing the framework.
//! Delivery and response registration initialize the delegate lazily.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use block2::RcBlock;
use futures::StreamExt as _;
use futures::channel::mpsc;
use gpui::{
    ForegroundExecutor, SharedString, SystemNotification, SystemNotificationAction,
    SystemNotificationResponse, Task,
};
use objc2::rc::Retained;
use objc2::runtime::{Bool, ProtocolObject};
use objc2::{AnyThread, DefinedClass, define_class, msg_send};
use objc2_foundation::{NSArray, NSBundle, NSError, NSObject, NSObjectProtocol, NSSet, NSString};
use objc2_user_notifications::{
    UNAuthorizationOptions, UNMutableNotificationContent, UNNotification, UNNotificationAction,
    UNNotificationActionOptions, UNNotificationCategory, UNNotificationCategoryOptions,
    UNNotificationDefaultActionIdentifier, UNNotificationPresentationOptions,
    UNNotificationRequest, UNNotificationResponse, UNNotificationSound, UNUserNotificationCenter,
    UNUserNotificationCenterDelegate, UNAuthorizationStatus, UNNotificationSettings,
};

fn center() -> Option<Retained<UNUserNotificationCenter>> {
    NSBundle::mainBundle().bundleIdentifier()?;
    Some(UNUserNotificationCenter::currentNotificationCenter())
}

fn authorization_status(value: UNAuthorizationStatus) -> gpui::SystemNotificationStatus {
    match value {
        UNAuthorizationStatus::Denied => gpui::SystemNotificationStatus::Denied,
        UNAuthorizationStatus::NotDetermined => gpui::SystemNotificationStatus::NotDetermined,
        _ => gpui::SystemNotificationStatus::Authorized,
    }
}

pub(crate) fn status(executor: &ForegroundExecutor) -> Task<anyhow::Result<gpui::SystemNotificationStatus>> {
    let Some(center) = center() else {
        return Task::ready(Ok(gpui::SystemNotificationStatus::Unsupported));
    };
    let (sender, receiver) = futures::channel::oneshot::channel();
    let sender = std::sync::Mutex::new(Some(sender));
    let completion = RcBlock::new(move |settings: std::ptr::NonNull<UNNotificationSettings>| {
        let value = authorization_status(unsafe { settings.as_ref() }.authorizationStatus());
        if let Some(sender) = sender.lock().unwrap().take() {
            let _ = sender.send(value);
        }
    });
    center.getNotificationSettingsWithCompletionHandler(&completion);
    executor.spawn(async move { receiver.await.map_err(|error| anyhow::anyhow!(error)) })
}

pub(crate) fn request(executor: &ForegroundExecutor) -> Task<anyhow::Result<gpui::SystemNotificationStatus>> {
    let Some(center) = center() else {
        return Task::ready(Ok(gpui::SystemNotificationStatus::Unsupported));
    };
    let (sender, receiver) = futures::channel::oneshot::channel();
    let sender = std::sync::Mutex::new(Some(sender));
    let completion = RcBlock::new(move |granted: Bool, error: *mut NSError| {
        let result = if let Some(error) = unsafe { error.as_ref() } {
            Err(error.localizedDescription().to_string())
        } else {
            Ok(if granted.as_bool() { gpui::SystemNotificationStatus::Authorized }
               else { gpui::SystemNotificationStatus::Denied })
        };
        if let Some(sender) = sender.lock().unwrap().take() {
            let _ = sender.send(result);
        }
    });
    center.requestAuthorizationWithOptions_completionHandler(
        UNAuthorizationOptions::Alert | UNAuthorizationOptions::Sound, &completion);
    executor.spawn(async move {
        receiver.await.map_err(|error| anyhow::anyhow!(error))?
            .map_err(|error| anyhow::anyhow!(error))
    })
}

type ResponseCallback = Rc<RefCell<Option<Box<dyn FnMut(SystemNotificationResponse)>>>>;

pub(crate) struct SystemNotificationState {
    initialized: bool,
    center: Option<NotificationCenter>,
    callback: ResponseCallback,
    _response_task: Option<Task<()>>,
}

impl SystemNotificationState {
    pub(crate) fn new() -> Self {
        Self {
            initialized: false,
            center: None,
            callback: Rc::new(RefCell::new(None)),
            _response_task: None,
        }
    }

    pub(crate) fn show(&mut self, executor: &ForegroundExecutor, notification: SystemNotification) {
        self.initialize(executor);
        if let Some(center) = &self.center {
            center.show(notification);
        }
    }

    pub(crate) fn show_with_options(
        &mut self, executor: &ForegroundExecutor, notification: SystemNotification,
        options: gpui::SystemNotificationOptions,
    ) {
        self.initialize(executor);
        if let Some(center) = &self.center {
            center.show_with_options(notification, options);
        }
    }

    pub(crate) fn dismiss(&mut self, executor: &ForegroundExecutor, tag: &str) {
        self.initialize(executor);
        if let Some(center) = &self.center {
            center.dismiss(tag);
        }
    }

    pub(crate) fn on_response(
        &mut self,
        executor: &ForegroundExecutor,
        callback: Box<dyn FnMut(SystemNotificationResponse)>,
    ) {
        self.initialize(executor);
        *self.callback.borrow_mut() = Some(callback);
    }

    fn initialize(&mut self, executor: &ForegroundExecutor) {
        if self.initialized {
            return;
        }
        self.initialized = true;

        let (sender, mut receiver) = mpsc::unbounded();
        self.center = NotificationCenter::new(sender);
        if self.center.is_none() {
            return;
        }

        // Responses arrive from the delegate on an arbitrary thread; this task
        // hands them to the registered callback on the main thread.
        let callback = self.callback.clone();
        self._response_task = Some(executor.spawn(async move {
            while let Some(response) = receiver.next().await {
                // Take the callback out for the call: it may re-enter the
                // platform (e.g. to dismiss the notification it was told
                // about) or replace itself.
                let taken = callback.borrow_mut().take();
                if let Some(mut taken) = taken {
                    taken(response);
                    callback.borrow_mut().get_or_insert(taken);
                }
            }
        }));
    }
}

struct NotificationCenter {
    center: Retained<UNUserNotificationCenter>,
    /// The center's `delegate` property is weak; keeping the delegate retained
    /// here keeps response handling alive for the app's lifetime.
    _delegate: Retained<NotificationResponseDelegate>,
    /// The union of every action set registered so far. Categories are shared
    /// by notifications with identical actions because macOS replaces the
    /// entire registered set whenever a category is added.
    categories: RefCell<
        HashMap<Vec<SystemNotificationAction>, (SharedString, Retained<UNNotificationCategory>)>,
    >,
    authorization_requested: Cell<bool>,
}

impl NotificationCenter {
    fn new(sender: mpsc::UnboundedSender<SystemNotificationResponse>) -> Option<Self> {
        // `UNUserNotificationCenter` raises `NSInternalInconsistencyException`
        // ("bundleProxyForCurrentProcess is nil"), aborting the process, when
        // the binary isn't part of an app bundle — which is how dev builds run
        // via `cargo run`. A bundle identifier is only present when launched
        // from a real `.app`, so use it as the guard.
        if NSBundle::mainBundle().bundleIdentifier().is_none() {
            log::info!("system notifications disabled: not running from an app bundle");
            return None;
        }

        let center = UNUserNotificationCenter::currentNotificationCenter();
        let delegate = NotificationResponseDelegate::new(sender);
        center.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));

        Some(Self {
            center,
            _delegate: delegate,
            categories: RefCell::new(HashMap::new()),
            authorization_requested: Cell::new(false),
        })
    }

    fn request_authorization(&self) {
        if self.authorization_requested.replace(true) {
            return;
        }
        let completion = RcBlock::new(|granted: Bool, error: *mut NSError| {
            // SAFETY: when non-null, `error` is a valid `NSError` for the
            // duration of the callback.
            if let Some(error) = unsafe { error.as_ref() } {
                log::warn!(
                    "system notification authorization failed: {}",
                    error.localizedDescription()
                );
            } else if !granted.as_bool() {
                log::info!("system notification authorization denied");
            }
        });
        self.center
            .requestAuthorizationWithOptions_completionHandler(
                UNAuthorizationOptions::Alert | UNAuthorizationOptions::Sound,
                &completion,
            );
    }

    fn show(&self, notification: SystemNotification) {
        self.request_authorization();
        self.show_with_options(notification, gpui::SystemNotificationOptions::default());
    }

    fn show_with_options(&self, notification: SystemNotification, options: gpui::SystemNotificationOptions) {
        let content = UNMutableNotificationContent::new();
        content.setSubtitle(&NSString::from_str(&options.subtitle));
        if options.sound {
            content.setSound(Some(&UNNotificationSound::defaultSound()));
        }
        content.setTitle(&NSString::from_str(&notification.title));
        content.setBody(&NSString::from_str(&notification.body));
        if !notification.actions.is_empty() {
            let category_identifier = self.register_category(&notification.actions);
            content.setCategoryIdentifier(&NSString::from_str(&category_identifier));
        }

        // A nil trigger delivers immediately. Reusing the tag as the request
        // identifier makes a newer notification for the same tag replace the
        // older one.
        let request = UNNotificationRequest::requestWithIdentifier_content_trigger(
            &NSString::from_str(&notification.tag),
            &content,
            None,
        );
        let completion = RcBlock::new(|error: *mut NSError| {
            // SAFETY: when non-null, `error` is a valid `NSError` for the
            // duration of the callback.
            if let Some(error) = unsafe { error.as_ref() } {
                log::warn!(
                    "failed to deliver system notification: {}",
                    error.localizedDescription()
                );
            }
        });
        self.center
            .addNotificationRequest_withCompletionHandler(&request, Some(&completion));
    }

    fn register_category(&self, actions: &[SystemNotificationAction]) -> SharedString {
        let mut categories = self.categories.borrow_mut();
        if let Some((identifier, _category)) = categories.get(actions) {
            return identifier.clone();
        }

        let identifier =
            SharedString::from(format!("gpui-system-notification-{}", categories.len()));
        let platform_actions: Vec<Retained<UNNotificationAction>> = actions
            .iter()
            .map(|action| {
                UNNotificationAction::actionWithIdentifier_title_options(
                    &NSString::from_str(&action.id),
                    &NSString::from_str(&action.label),
                    UNNotificationActionOptions::empty(),
                )
            })
            .collect();
        let category =
            UNNotificationCategory::categoryWithIdentifier_actions_intentIdentifiers_options(
                &NSString::from_str(&identifier),
                &NSArray::from_retained_slice(&platform_actions),
                &NSArray::new(),
                UNNotificationCategoryOptions::empty(),
            );

        categories.insert(actions.to_vec(), (identifier.clone(), category));
        let all: Vec<Retained<UNNotificationCategory>> = categories
            .values()
            .map(|(_identifier, category)| category.clone())
            .collect();
        self.center
            .setNotificationCategories(&NSSet::from_retained_slice(&all));
        identifier
    }

    fn dismiss(&self, tag: &str) {
        let identifiers = NSArray::from_retained_slice(&[NSString::from_str(tag)]);
        self.center
            .removePendingNotificationRequestsWithIdentifiers(&identifiers);
        self.center
            .removeDeliveredNotificationsWithIdentifiers(&identifiers);
    }
}

struct DelegateIvars {
    sender: mpsc::UnboundedSender<SystemNotificationResponse>,
}

define_class!(
    // SAFETY: `NSObject` has no subclassing requirements and
    // `NotificationResponseDelegate` does not implement `Drop`.
    #[unsafe(super(NSObject))]
    #[ivars = DelegateIvars]
    struct NotificationResponseDelegate;

    unsafe impl NSObjectProtocol for NotificationResponseDelegate {}

    unsafe impl UNUserNotificationCenterDelegate for NotificationResponseDelegate {
        // Called when the user activates a delivered notification, possibly on
        // a non-main thread; the channel hop hands the response to the
        // foreground task holding the receiver.
        #[unsafe(method(userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:))]
        fn did_receive_notification_response(
            &self,
            _center: &UNUserNotificationCenter,
            response: &UNNotificationResponse,
            completion_handler: &block2::DynBlock<dyn Fn()>,
        ) {
            let tag = response.notification().request().identifier().to_string();
            let action = response.actionIdentifier();
            // The other well-known identifier, `UNNotificationDismissActionIdentifier`,
            // is only delivered for categories opting into dismiss callbacks,
            // which we never request.
            let action_id = if &*action == unsafe { UNNotificationDefaultActionIdentifier } {
                None
            } else {
                Some(SharedString::from(action.to_string()))
            };
            self.ivars()
                .sender
                .unbounded_send(SystemNotificationResponse {
                    tag: SharedString::from(tag),
                    action_id,
                })
                .ok();
            completion_handler.call(());
        }

        // Without this, macOS suppresses banners while the app is frontmost.
        // Posting is the application's decision; whether the app is focused is
        // its criterion to apply, not the platform's.
        #[unsafe(method(userNotificationCenter:willPresentNotification:withCompletionHandler:))]
        fn will_present_notification(
            &self,
            _center: &UNUserNotificationCenter,
            notification: &UNNotification,
            completion_handler: &block2::DynBlock<dyn Fn(UNNotificationPresentationOptions)>,
        ) {
            let mut options = UNNotificationPresentationOptions::Banner
                | UNNotificationPresentationOptions::List;
            if notification.request().content().sound().is_some() {
                options |= UNNotificationPresentationOptions::Sound;
            }
            completion_handler.call((options,));
        }
    }
);

impl NotificationResponseDelegate {
    fn new(sender: mpsc::UnboundedSender<SystemNotificationResponse>) -> Retained<Self> {
        let this = Self::alloc().set_ivars(DelegateIvars { sender });
        // SAFETY: `NSObject`'s `init` is its designated initializer.
        unsafe { msg_send![super(this), init] }
    }
}
