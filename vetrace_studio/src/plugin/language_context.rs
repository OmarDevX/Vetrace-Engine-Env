use super::*;

pub(super) fn language_context(engine: &Engine, project: &VetraceProject) -> vetrace_script_editor::LanguageContext {
    let mut components = engine.registered_component_schemas()
        .into_iter()
        .filter(|schema| schema.lua_accessible)
        .map(|schema| {
            let compact_name = schema.display_name.chars().filter(|character| !character.is_whitespace()).collect::<String>();
            vetrace_script_editor::CompletionComponent {
                stable_id: schema.stable_id,
                display_name: schema.display_name,
                aliases: vec![compact_name],
                fields: schema.fields.into_iter().map(|field| field.name).collect(),
            }
        })
        .collect::<Vec<_>>();
    components.sort_by(|left, right| left.display_name.cmp(&right.display_name));
    vetrace_script_editor::LanguageContext {
        components,
        input_actions: project.manifest().input.actions.keys().cloned().collect(),
    }
}
