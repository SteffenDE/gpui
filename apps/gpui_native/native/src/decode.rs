use crate::{decode_element_node, ElementNode};
use rustler::{NifResult, OwnedEnv, Term};

// Recursive schema decoding has large style/variant frames. BEAM dirty IO
// scheduler stacks are much smaller than Rust UI-thread stacks. Copy the term
// into an owned environment and decode on a bounded, explicitly sized stack.
// No process-bound Env/Term is transferred across threads.
pub(crate) fn decode_tree(tree: Term<'_>) -> NifResult<ElementNode> {
    let mut pending = vec![(tree, 0usize)];
    while let Some((node, depth)) = pending.pop() {
        if depth > 128 {
            return Err(rustler::Error::Term(Box::new("native_tree_too_deep")));
        }
        if let Ok(children) = node.map_get(crate::atoms::children()) {
            if let Ok(children) = children.decode::<Vec<Term<'_>>>() {
                pending.extend(children.into_iter().map(|child| (child, depth + 1)));
            }
        }
    }
    let owned = OwnedEnv::new();
    let saved = owned.save(tree);
    std::thread::Builder::new()
        .name("gpui-decode".into())
        .stack_size(8 * 1024 * 1024)
        .spawn(move || owned.run(|env| decode_element_node(saved.load(env)).map_err(|_| ())))
        .map_err(|_| rustler::Error::Atom("native_decode_worker_unavailable"))?
        .join()
        .map_err(|_| rustler::Error::Atom("native_decode_worker_failed"))?
        .map_err(|_| rustler::Error::BadArg)
}
