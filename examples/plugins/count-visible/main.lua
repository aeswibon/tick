-- Show how many filtered rows are visible (Ctrl+Shift+C).
-- Install: cp -R examples/plugins/count-visible ~/.config/tick/plugins/

function on_key(chord)
  if chord ~= "ctrl+shift+c" then
    return "passthrough"
  end
  local n = 0
  if tick and tick.tickets then
    n = #tick.tickets
  end
  local view = tick and tick.view and tick.view.name or "view"
  tick._notice = string.format("%s: %d visible row(s)", view, n)
  return "handled"
end
