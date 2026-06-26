local label
local editor
local current = ""

function start(engine, self)
    label = label or engine:find_entity_by_name("Label")
    editor = editor or engine:find_entity_by_name("Editor")

    -- debug: show which entity this script is attached to
    local meta = self.Metadata or { name = "(none)" }
    engine:print("ui_example start for " .. meta.name)

    if editor then
        editor:subscribe_event("changed", function(text)
            current = text
            engine:print("editor changed to " .. text)
        end)
    end

    if meta and meta.name == "Button" then
        engine:print("subscribing click for Button")
        self:subscribe_event("clicked", function()
            engine:print("submit clicked")
            if label and label.UILabel then
                label.UILabel.text = current
                engine:request_redraw()
            end
        end)
    elseif meta and meta.name == "ClearButton" then
        engine:print("subscribing click for ClearButton")
        self:subscribe_event("clicked", function()
            engine:print("clear clicked")
            current = ""
            if label and label.UILabel then
                label.UILabel.text = ""
                engine:request_redraw()
            end
            if editor and editor.UITextEditor then
                editor.UITextEditor.text = ""
            end
        end)
    end
end

function update(engine, self, input, dt)
end
