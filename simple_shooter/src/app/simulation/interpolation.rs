use super::*;

pub(crate) fn update_client_interpolation(engine: &mut Engine, client: &mut ClientState, dt: f32) {
    let component_name = TransformReplicator::component_name();
    let ids = client.transform_interpolation.ids_for(component_name);
    for id in ids {
        let Some(actor) = client.net.mapped_actor(id) else { continue; };
        if Some(id) == client.net.client_id() { continue; }
        let mut should_remove = false;
        if let Some(state) = client.transform_interpolation.get_mut(id, component_name) {
            let (from, to, alpha, complete) = state.advance_cloned(dt);
            let snapshot = TransformReplicator::interpolate_snapshot(&from, &to, alpha);
            TransformReplicator::apply_snapshot_to_actor(engine, actor, &snapshot);
            should_remove = complete;
        }
        if should_remove {
            client.transform_interpolation.remove(id, component_name);
        }
    }
}
