-- ClassicAPI's function containers and event callbacks (`event/FunctionContainer.cpp`,
-- `event/Callback.cpp`): `C_FunctionContainers.CreateCallback`, the globals
-- `RegisterEventCallback`, `UnregisterEventCallback`, `RegisterUnitEventCallback`,
-- `UnregisterUnitEventCallback`, the frame methods `RegisterEventCallback` and
-- `RegisterUnitEventCallback`, and `C_EventUtils.IsCallbackEvent`.
--
-- Only the eight callback events take callbacks. A callback gets its owner (nil for a global
-- registration, the frame for a frame's) and the event's values, never the name, protected under
-- the error handler, after every frame's OnEvent: the global callbacks, then the frames'. A
-- registration or removal made while the event fires applies when that fire ends. Two
-- registrations match only when they hold the same container, so a plain function registered
-- twice runs twice and unregistering one removes nothing. The engine calls the returned
-- dispatch after the frame walk (`pfui_host::event_callbacks`).
local tables = ...

local NAMES = {
  "MINIMAP_PING", "COMBAT_LOG_APPLY_FILTER_SETTINGS", "ENCOUNTER_STATE_CHANGED",
  "TOOLTIP_SHOW_ITEM_COMPARISON", "COMBAT_LOG_EVENT", "COMBAT_LOG_EVENT_UNFILTERED",
  "COMBAT_LOG_EVENT_INTERNAL_UNFILTERED", "COMBAT_LOG_REFILTER_ENTRIES",
}
local ID = {}
for _, name in ipairs(NAMES) do ID[name] = name end
local function event_id(name)
  return type(name) == "string" and ID[string.upper(name)] or nil
end
-- The combat log events carry no values: a callback reads CombatLogGetCurrentEventInfo().
local NO_VALUES = {
  COMBAT_LOG_EVENT = true, COMBAT_LOG_EVENT_UNFILTERED = true,
  COMBAT_LOG_EVENT_INTERNAL_UNFILTERED = true,
}

-- Containers ------------------------------------------------------------------------------------

local Container = {}
local container_meta = { __index = Container }
local containers = setmetatable({}, { __mode = "k" })

local function report(err)
  local handler = geterrorhandler and geterrorhandler()
  if handler then handler(err) end
end

function Container:Invoke(...)
  local func = containers[self]
  if type(func) ~= "function" then return end
  local ok, err = pcall(func, unpack(arg, 1, arg.n))
  if not ok then report(err) end
end
function Container:Cancel()
  if containers[self] == nil then error("Usage: callback:Cancel()", 2) end
  containers[self] = false
end
function Container:IsCancelled()
  if containers[self] == nil then error("Usage: callback:IsCancelled()", 2) end
  return containers[self] == false
end

local function new_container(func)
  local c = setmetatable({}, container_meta)
  containers[c] = func
  return c
end

C_FunctionContainers = C_FunctionContainers or {}
function C_FunctionContainers.CreateCallback(func)
  if type(func) ~= "function" then error("Usage: C_FunctionContainers.CreateCallback(func)", 2) end
  return new_container(func)
end

-- A callback argument as the container it registers: a function gets a container of its own.
local function container_of(value)
  if type(value) == "function" then return new_container(value) end
  if type(value) == "table" and containers[value] ~= nil then return value end
end

-- Registries ------------------------------------------------------------------------------------

local registries = {}
for _, name in ipairs(NAMES) do registries[name] = { entries = {}, pending = {}, depth = 0 } end

local function same(a, b) return a.c == b.c and a.frame == b.frame and a.unit == b.unit end

local function insert(r, entry)
  for _, e in ipairs(r.entries) do if same(e, entry) then return end end
  table.insert(r.entries, entry)
end
local function erase(r, entry)
  for i, e in ipairs(r.entries) do
    if same(e, entry) then table.remove(r.entries, i) return end
  end
end
local function add(name, entry)
  local r = registries[name]
  if r.depth > 0 then table.insert(r.pending, { true, entry }) else insert(r, entry) end
end
local function remove(name, entry)
  local r = registries[name]
  if r.depth > 0 then table.insert(r.pending, { false, entry }) else erase(r, entry) end
end
local function remove_frame(name, frame)
  local r, matches = registries[name], {}
  for _, e in ipairs(r.entries) do if e.frame == frame then table.insert(matches, e) end end
  for _, op in ipairs(r.pending) do
    if op[1] and op[2].frame == frame then table.insert(matches, op[2]) end
  end
  for _, e in ipairs(matches) do remove(name, e) end
end

-- One unit token, lowercased; nil for a value that is none.
local SIMPLE = { player = true, pet = true, target = true, focus = true, mouseover = true }
local NUMBERED = { party = 4, partypet = 4, raid = 40, raidpet = 40, boss = 5, nameplate = 40 }
local function unit_token(value)
  if type(value) ~= "string" then return nil end
  local token = string.lower(value)
  if SIMPLE[token] then return token end
  local _, _, base, n = string.find(token, "^(%a+)(%d+)$")
  n = tonumber(n)
  if base and NUMBERED[base] and n >= 1 and n <= NUMBERED[base] then return token end
end

local function unknown(fn, verb, name)
  error(fn .. ' Attempt to ' .. verb .. ' unknown event "' .. tostring(name or "") .. '"', 3)
end

local function global_register(fn, verb, adding, with_unit, name, callback, unit)
  local id = event_id(name)
  if not id or (with_unit and id ~= "MINIMAP_PING") then unknown(fn, verb, name) end
  local c = container_of(callback)
  if not c then error("bad argument #2 to '" .. fn .. "' (invalid callback)", 3) end
  local token = ""
  if with_unit then
    token = unit_token(unit)
    if not token then unknown(fn, verb, name) end
  end
  local entry = { c = c, unit = token }
  if adding then add(id, entry) else remove(id, entry) end
  return true
end

function RegisterEventCallback(name, callback)
  return global_register("RegisterEventCallback", "register", true, false, name, callback)
end
function UnregisterEventCallback(name, callback)
  return global_register("UnregisterEventCallback", "register", false, false, name, callback)
end
function RegisterUnitEventCallback(name, callback, unit)
  return global_register("RegisterUnitEventCallback", "register", true, true, name, callback, unit)
end
function UnregisterUnitEventCallback(name, callback, unit)
  return global_register("UnregisterUnitEventCallback", "unregister", false, true, name, callback,
    unit)
end

C_EventUtils = C_EventUtils or {}
function C_EventUtils.IsCallbackEvent(name)
  if type(name) ~= "string" and type(name) ~= "number" then
    error("bad argument #1 to 'IsCallbackEvent' (Usage: local isCallbackEvent = C_EventUtils.IsCallbackEvent(eventName))", 2)
  end
  return event_id(tostring(name)) ~= nil
end

-- Frames: their own registrations, removed by UnregisterEvent and UnregisterAllEvents.
local frame = tables.frame
local function frame_register(method, with_unit)
  return function(self, name, callback, unit)
    local id = event_id(name)
    local known = id and (not with_unit or id == "MINIMAP_PING")
    local c = known and container_of(callback)
    if known and not c then error("bad argument #3 to '" .. method .. "' (invalid callback)", 2) end
    local token = ""
    if with_unit and known then token = unit_token(unit) end
    if not known or not token then
      local owner = self:GetName() or self:GetObjectType()
      error(owner .. ":" .. method .. '(): Attempt to register unknown event "' .. tostring(name or "") .. '"', 2)
    end
    add(id, { c = c, frame = self, unit = token })
    return true
  end
end
frame.RegisterEventCallback = frame_register("RegisterEventCallback", false)
frame.RegisterUnitEventCallback = frame_register("RegisterUnitEventCallback", true)
local unregister_event, unregister_all = frame.UnregisterEvent, frame.UnregisterAllEvents
frame.UnregisterEvent = function(self, name, a1, a2, a3)
  local id = event_id(name)
  if id then remove_frame(id, self) end
  return unregister_event(self, name, a1, a2, a3)
end
frame.UnregisterAllEvents = function(self, a1, a2, a3)
  for _, name in ipairs(NAMES) do remove_frame(name, self) end
  return unregister_all(self, a1, a2, a3)
end

-- Dispatch --------------------------------------------------------------------------------------

local function invoke(entry, values)
  local func = containers[entry.c]
  if type(func) ~= "function" then return end
  local ok, err = pcall(func, entry.frame, unpack(values, 1, values.n))
  if not ok then report(err) end
end

local function run_pass(r, unit, values)
  for frame_pass = 0, 1 do
    local i = 1
    while r.entries[i] do
      local e = r.entries[i]
      if (e.frame ~= nil) == (frame_pass == 1) and e.unit == (unit or "") then
        invoke(e, values)
      end
      i = i + 1
    end
  end
end

-- The callbacks of `name` after its frame walk: the plain ones, then, for MINIMAP_PING, those of
-- the unit that sent it.
return function(name, ...)
  local r = registries[name]
  if not r or (table.getn(r.entries) == 0 and r.depth == 0) then return end
  local values = NO_VALUES[name] and { n = 0 } or arg
  r.depth = r.depth + 1
  run_pass(r, nil, values)
  local unit = name == "MINIMAP_PING" and unit_token(arg[1])
  if unit then run_pass(r, unit, values) end
  r.depth = r.depth - 1
  if r.depth == 0 and table.getn(r.pending) > 0 then
    local ops = r.pending
    r.pending = {}
    for _, op in ipairs(ops) do
      if op[1] then insert(r, op[2]) else erase(r, op[2]) end
    end
  end
end
