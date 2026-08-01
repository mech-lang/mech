use super::new_runtime;
use crate::{ObjectId, ObjectRecord};

#[test]
fn active_transaction_must_continue_with_original_context() {
    let mut runtime = new_runtime();
    let mut context = runtime.runtime_context().unwrap();
    context.subject = "owner".to_string();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime
        .put_object_with_context(
            &mut context,
            ObjectRecord::text(ObjectId(906), "note", "staged"),
        )
        .unwrap();
    assert!(runtime.get_object(ObjectId(906)).unwrap().is_none());
    assert_eq!(
        runtime.commit_runtime_transaction(&mut context).unwrap(),
        transaction_id,
    );
    assert!(runtime.get_object(ObjectId(906)).unwrap().is_some());
}
