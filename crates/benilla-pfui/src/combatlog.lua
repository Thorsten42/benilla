-- The combat log API a ClassicAPI client gives addons. Until the client feeds its combat events,
-- the log holds no entry and COMBAT_LOG_EVENT_UNFILTERED never fires, so an addon reading it sees
-- an empty log rather than a missing API.
local filters = {}
function CombatLogResetFilter() filters = {} end
function CombatLogAddFilter(events, source, destination)
  tinsert(filters, {events, source, destination})
end
function CombatLogGetNumEntries() return 0 end
function CombatLogGetCurrentEntry() return nil end
function CombatLogSetCurrentEntry(index) return false end
function CombatLogAdvanceEntry(count) return false end
function CombatLogClearEntries() end
function CombatLogGetRetentionTime() return 300 end
function CombatLogSetRetentionTime(seconds) end
function CombatLogGetCurrentEventInfo() return nil end
