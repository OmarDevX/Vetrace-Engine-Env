use std::collections::BTreeSet;

use vetrace_script_editor::{
    CompletionContext, CompletionItem, CompletionKind, SymbolKind,
};

use crate::{lua_symbols, KEYWORDS};

pub(crate) fn lua_completions(context: CompletionContext<'_>) -> Vec<CompletionItem> {
    let cursor = context.cursor_byte.min(context.source.len());
    let prefix = &context.source[..cursor];
    if let Some(component_prefix) = prefix.rsplit_once("self.components.").map(|(_, tail)| tail) {
        if let Some((component_name, field_prefix)) = component_prefix.rsplit_once('.') {
            if !component_name.contains(|character: char| character.is_whitespace() || character == '(') {
                if let Some(component) = context.language.components.iter().find(|component| {
                    component.display_name == component_name
                        || component.stable_id == component_name
                        || component.aliases.iter().any(|alias| alias == component_name)
                }) {
                    return component.fields.iter()
                        .filter(|field| field.starts_with(field_prefix))
                        .map(|field| CompletionItem {
                            label: field.clone(),
                            insert_text: field[field_prefix.len()..].to_owned(),
                            detail: format!("{} field", component.display_name),
                            kind: CompletionKind::Property,
                        })
                        .collect();
                }
            }
        }
        let typed = component_prefix.rsplit(|character: char| !character.is_alphanumeric() && character != '_').next().unwrap_or("");
        return context.language.components.iter().filter_map(|component| {
            let name = component.aliases.first().cloned().unwrap_or_else(|| component.display_name.replace(' ', ""));
            name.starts_with(typed).then(|| CompletionItem {
                label: name.clone(),
                insert_text: name[typed.len()..].to_owned(),
                detail: component.stable_id.clone(),
                kind: CompletionKind::Component,
            })
        }).collect();
    }

    for function in ["Input.action_down(\"", "Input.action_pressed(\"", "Input.action_released(\""] {
        if let Some(action_prefix) = prefix.rsplit_once(function).map(|(_, tail)| tail) {
            if !action_prefix.contains('"') {
                return context.language.input_actions.iter()
                    .filter(|action| action.starts_with(action_prefix))
                    .map(|action| CompletionItem {
                        label: action.clone(),
                        insert_text: format!("{}\")", &action[action_prefix.len()..]),
                        detail: "Project input action".into(),
                        kind: CompletionKind::InputAction,
                    })
                    .collect();
            }
        }
    }

    let typed = prefix.rsplit(|character: char| !character.is_alphanumeric() && character != '_').next().unwrap_or("");
    let mut items = Vec::new();
    for keyword in KEYWORDS {
        if keyword.starts_with(typed) && *keyword != typed {
            items.push(CompletionItem {
                label: (*keyword).into(),
                insert_text: keyword[typed.len()..].into(),
                detail: "Lua keyword".into(),
                kind: CompletionKind::Keyword,
            });
        }
    }
    for (label, insertion, detail) in [
        ("ready", "ready = function(self)\n    \nend,", "Vetrace lifecycle callback"),
        ("update", "update = function(self, dt)\n    \nend,", "Vetrace lifecycle callback"),
        ("fixed_update", "fixed_update = function(self, dt)\n    \nend,", "Vetrace lifecycle callback"),
        ("Scene.spawn", "Scene.spawn(\"\")", "Spawn an entity"),
        ("Debug.log", "Debug.log(\"\")", "Write to game output"),
        ("Input.action_down", "Input.action_down(\"\")", "Read a project input action"),
        ("Input.mouse_button_pressed", "Input.mouse_button_pressed(\"Left\")", "Read a mouse press transition"),
        ("Scene.find_by_name", "Scene.find_by_name(\"\")", "Find an entity by name"),
        ("Scene.find_by_tag", "Scene.find_by_tag(\"\")", "Find an entity by tag"),
        ("Scene.find_all_by_tag", "Scene.find_all_by_tag(\"\")", "Find all entities with a tag"),
        ("Physics.set_enabled", "Physics.set_enabled(entity, true)", "Enable or disable entity physics"),
        ("Physics.is_enabled", "Physics.is_enabled(entity)", "Check whether entity physics is enabled"),
        ("entity:is_spawned", "entity:is_spawned()", "Check whether a deferred entity handle has resolved"),
        ("UI.button", "UI.button()", "Read a screen-space button interaction"),
        ("Application.quit", "Application.quit()", "Stop the running game"),
        ("Window.set_cursor", "Window.set_cursor(true, false)", "Set cursor visibility and grab mode"),
        ("Rendering.get", "Rendering.get(\"vsync\")", "Read a runtime render setting"),
        ("Rendering.set", "Rendering.set(\"vsync\", true)", "Change a runtime render setting"),
        ("Storage.write_text", "Storage.write_text(\"settings.json\", \"\")", "Write project-local user data"),
        ("Modules.require", "Modules.require(\"assets/scripts/module.lua\")", "Load a cached project-local Lua module"),
        ("Modules.invalidate", "Modules.invalidate(\"assets/scripts/module.lua\")", "Invalidate a cached Lua module"),
        ("Json.encode", "Json.encode({})", "Encode a Lua value as JSON"),
        ("Net.open", "Net.open(\"game\", \"0.0.0.0:0\")", "Open a generic UDP channel"),
    ] {
        if label.starts_with(typed) {
            let insert_text = insertion
                .strip_prefix(typed)
                .unwrap_or(insertion)
                .to_owned();
            items.push(CompletionItem {
                label: label.into(),
                insert_text,
                detail: detail.into(),
                kind: CompletionKind::Snippet,
            });
        }
    }
    let mut seen = items.iter().map(|item| item.label.clone()).collect::<BTreeSet<_>>();
    for symbol in lua_symbols(context.source) {
        if symbol.name.starts_with(typed) && symbol.name != typed && seen.insert(symbol.name.clone()) {
            items.push(CompletionItem {
                label: symbol.name.clone(),
                insert_text: symbol.name[typed.len()..].to_owned(),
                detail: match symbol.kind {
                    SymbolKind::Function => "Local function",
                    SymbolKind::Parameter => "Function parameter",
                    SymbolKind::Property => "Script property",
                    SymbolKind::Module => "Module",
                    SymbolKind::Local => "Local variable",
                }.into(),
                kind: match symbol.kind {
                    SymbolKind::Function => CompletionKind::Function,
                    _ => CompletionKind::Property,
                },
            });
        }
    }
    items.sort_by(|left, right| {
        completion_rank(left, typed)
            .cmp(&completion_rank(right, typed))
            .then_with(|| left.label.to_ascii_lowercase().cmp(&right.label.to_ascii_lowercase()))
    });
    items
}

fn completion_rank(item: &CompletionItem, typed: &str) -> (u8, usize) {
    let kind = match item.kind {
        CompletionKind::Property | CompletionKind::Method => 0,
        CompletionKind::Function => 1,
        CompletionKind::Component | CompletionKind::InputAction => 2,
        CompletionKind::Snippet => 3,
        CompletionKind::Keyword => 4,
    };
    let prefix_penalty = if item.label.starts_with(typed) { 0 } else { 1 };
    (kind + prefix_penalty, item.label.len())
}
