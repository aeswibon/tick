-- Drop Epics from the active view after each refresh.
-- Install: cp -R examples/plugins/hide-epics ~/.config/tick/plugins/

function filter_tickets(tickets)
  local out = {}
  for _, t in ipairs(tickets) do
    if t.issue_type ~= "Epic" then
      table.insert(out, t)
    end
  end
  return out
end
