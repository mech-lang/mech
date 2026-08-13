use mech_engine::resident::ReactiveInstance;

fn decompose(instance: ReactiveInstance) {
    let (_instance, _authority) = instance.into_coordinator_parts();
}

fn main() {}
