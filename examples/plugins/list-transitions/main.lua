-- List workflow transitions for the selected issue (Ctrl+Shift+T).
-- Use an id with tick.run_transition(key, id) from another plugin or script.
-- Install: cp -R examples/plugins/list-transitions ~/.config/tick/plugins/

function on_key(chord)
  if chord ~= "ctrl+shift+t" then
    return "passthrough"
  end
  local sel = tick.selected
  if not sel then
    tick._notice = "Select a row first"
    return "handled"
  end
  local list = tick.list_transitions(sel.key)
  if #list == 0 then
    tick._notice = sel.key .. ": no transitions"
    return "handled"
  end
  local parts = {}
  for _, t in ipairs(list) do
    table.insert(parts, t.id .. " " .. t.name .. "→" .. t.to_status)
  end
  tick._notice = sel.key .. ": " .. table.concat(parts, " | ")
  return "handled"
end
