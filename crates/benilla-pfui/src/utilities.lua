-- Independently implemented pfUI utility subset. Keep legacy FrameXML's globals usable.
-- The VM's legacy vararg table retains its argument count, including trailing nils.
function SafePack(...) return arg end

-- Friend-note buttons use this ClassicAPI utility as their OnLeave callback.
function GameTooltip_Hide()
  GameTooltip:Hide()
end

function Mixin(object, ...)
  for i = 1, arg.n do
    for key, value in pairs(arg[i]) do object[key] = value end
  end
  return object
end

function CreateFromMixins(...)
  return Mixin({}, unpack(arg, 1, arg.n))
end

function CreateAndInitFromMixin(mixin, ...)
  local object = Mixin({}, mixin)
  object:Init(unpack(arg, 1, arg.n))
  return object
end

function tInvert(values)
  local inverted = {}
  for key, value in pairs(values) do inverted[value] = key end
  return inverted
end

ColorMixin = {}
function ColorMixin:GetRGB() return self.r, self.g, self.b end
function ColorMixin:GetRGBA() return self.r, self.g, self.b, self.a end
function ColorMixin:SetRGB(r, g, b) self.r, self.g, self.b = r, g, b end
function ColorMixin:SetRGBA(r, g, b, a) self.r, self.g, self.b, self.a = r, g, b, a end
local function byte(value)
  return math.floor(math.max(0, math.min(1, value)) * 255)
end
function ColorMixin:GenerateHexColor()
  return string.format("%02x%02x%02x%02x", byte(self.a or 1), byte(self.r), byte(self.g), byte(self.b))
end
function ColorMixin:GenerateHexColorMarkup() return "|c" .. self:GenerateHexColor() end
function ColorMixin:WrapTextInColorCode(text) return self:GenerateHexColorMarkup() .. text .. "|r" end
function CreateColor(r, g, b, a)
  return Mixin({r = r, g = g, b = b, a = a or 1}, ColorMixin)
end
C_ColorUtil = {}
-- An engine function: it reads the color's fields itself, as ClassicAPI's ColorMixin calls it.
function C_ColorUtil.GenerateTextColorCode(color)
  return string.format("%02x%02x%02x%02x", byte(color.a or 1), byte(color.r), byte(color.g), byte(color.b))
end
-- ClassicAPI's text wrappers (color/Util.cpp): "|c" .. code .. text .. "|r", and the same with the
-- code a color generates.
function C_ColorUtil.WrapTextInColorCode(text, code)
  return "|c" .. tostring(code) .. tostring(text) .. "|r"
end
function C_ColorUtil.WrapTextInColor(text, color)
  return C_ColorUtil.WrapTextInColorCode(text, C_ColorUtil.GenerateTextColorCode(color))
end

-- ClassicAPI's color-space conversions (color/Util.cpp). Hue is in degrees, -1 for an achromatic
-- color, which every conversion passes through; each argument must be a number.
local function color_args(usage, a, b, c)
  a, b, c = tonumber(a), tonumber(b), tonumber(c)
  if not (a and b and c) then error(usage, 3) end
  return a, b, c
end
local function fmod(a, b) return a - b * (a >= 0 and math.floor(a / b) or math.ceil(a / b)) end
local function min_luma(l) return math.min(l, 1 - l) end
local function sector(c, x, h)
  local i = math.mod(math.floor(h), 6)
  if i == 0 then return c, x, 0 elseif i == 1 then return x, c, 0 elseif i == 2 then return 0, c, x
  elseif i == 3 then return 0, x, c elseif i == 4 then return x, 0, c end
  return c, 0, x
end
function C_ColorUtil.ConvertRGBToHSV(r, g, b)
  r, g, b = color_args("Usage: C_ColorUtil.ConvertRGBToHSV(r, g, b)", r, g, b)
  local mx, mn = math.max(r, g, b), math.min(r, g, b)
  local d = mx - mn
  local s = mx <= 0 and 0 or d / mx
  if d <= 0 then return -1, s, mx end
  local h
  if mx == r then h = 60 * fmod((g - b) / d, 6)
  elseif mx == g then h = 60 * ((b - r) / d + 2)
  else h = 60 * ((r - g) / d + 4) end
  if h < 0 then h = h + 360 end
  return h, s, mx
end
function C_ColorUtil.ConvertHSVToRGB(h, s, v)
  h, s, v = color_args("Usage: C_ColorUtil.ConvertHSVToRGB(h, s, v)", h, s, v)
  if s <= 0 or h < 0 then return v, v, v end
  h = fmod(h, 360)
  if h < 0 then h = h + 360 end
  local hh = h / 60
  local f = hh - math.floor(hh)
  local p, q, t = v * (1 - s), v * (1 - s * f), v * (1 - s * (1 - f))
  local i = math.mod(math.floor(hh), 6)
  if i == 0 then return v, t, p elseif i == 1 then return q, v, p elseif i == 2 then return p, v, t
  elseif i == 3 then return p, q, v elseif i == 4 then return t, p, v end
  return v, p, q
end
function C_ColorUtil.ConvertHSVToHSL(h, s, v)
  h, s, v = color_args("Usage: C_ColorUtil.ConvertHSVToHSL(h, s, v)", h, s, v)
  local l = v * (1 - s / 2)
  return h, (l <= 0 or l >= 1) and 0 or (v - l) / min_luma(l), l
end
function C_ColorUtil.ConvertHSLToHSV(h, s, l)
  h, s, l = color_args("Usage: C_ColorUtil.ConvertHSLToHSV(h, s, l)", h, s, l)
  local v = l + s * min_luma(l)
  return h, v <= 0 and 0 or 2 * (1 - l / v), v
end
function C_ColorUtil.ConvertHSLToRGB(h, s, l)
  h, s, l = color_args("Usage: C_ColorUtil.ConvertHSLToRGB(h, s, l)", h, s, l)
  if s <= 0 or h < 0 then return l, l, l end
  h = fmod(h, 360)
  if h < 0 then h = h + 360 end
  local c = (1 - math.abs(2 * l - 1)) * s
  local hh = h / 60
  local x = c * (1 - math.abs(fmod(hh, 2) - 1))
  local r1, g1, b1 = sector(c, x, hh)
  local m = l - c / 2
  return r1 + m, g1 + m, b1 + m
end

-- ClassicAPI's `C_Glue.IsOnGlueScreen()` (glue/Session.cpp): fixed per Lua state, false in the
-- world's, where this layer runs.
C_Glue = C_Glue or {}
function C_Glue.IsOnGlueScreen() return false end

CallbackRegistryMixin = {}
function CallbackRegistryMixin:OnLoad()
  self.callbacks, self.definedEvents = {}, {}
end
function CallbackRegistryMixin:SetUndefinedEventsAllowed(allowed)
  self.undefinedEventsAllowed = allowed
end
function CallbackRegistryMixin:GenerateCallbackEvents(events)
  for _, name in ipairs(events) do self.definedEvents[name] = true end
end
function CallbackRegistryMixin:RegisterCallback(name, callback, owner, ...)
  assert(self.undefinedEventsAllowed or self.definedEvents[name], "undefined callback event: " .. name)
  assert(type(callback) == "function", "callback must be a function")
  local entries = self.callbacks[name] or {}
  self.callbacks[name] = entries
  local entry = {callback = callback, owner = owner, extra = arg}
  table.insert(entries, entry)
  return entry
end
function CallbackRegistryMixin:UnregisterCallback(name, owner)
  local entries = self.callbacks[name] or {}
  for i = table.getn(entries), 1, -1 do
    if entries[i].owner == owner then
      entries[i].removed = true
      table.remove(entries, i)
    end
  end
end
function CallbackRegistryMixin:UnregisterAllCallbacks(owner)
  for name in pairs(self.callbacks) do self:UnregisterCallback(name, owner) end
end
function CallbackRegistryMixin:TriggerEvent(name, ...)
  local snapshot = {}
  for _, entry in ipairs(self.callbacks[name] or {}) do table.insert(snapshot, entry) end
  for _, entry in ipairs(snapshot) do
    if not entry.removed then
      local values = {entry.owner}
      local n = 1
      for i = 1, entry.extra.n do n = n + 1; values[n] = entry.extra[i] end
      for i = 1, arg.n do n = n + 1; values[n] = arg[i] end
      entry.callback(unpack(values, 1, n))
    end
  end
end

-- Player-login readiness belongs to this VM; it is reset when the UI is rebuilt.
local loggedIn = false
local loginFrame = CreateFrame("Frame")
loginFrame:RegisterEvent("PLAYER_LOGIN")
loginFrame:SetScript("OnEvent", function()
  loggedIn = true
  loginFrame:UnregisterEvent("PLAYER_LOGIN")
end)
function IsLoggedIn() return loggedIn end
EventUtil = {}
local function continueWhen(eventName, ready, callback, matches)
  if ready then callback(); return end
  local frame = CreateFrame("Frame")
  frame:RegisterEvent(eventName)
  frame:SetScript("OnEvent", function()
    if not matches or matches(arg1) then
      frame:UnregisterEvent(eventName)
      frame:SetScript("OnEvent", nil)
      callback()
    end
  end)
end
function EventUtil.ContinueOnPlayerLogin(callback)
  continueWhen("PLAYER_LOGIN", loggedIn, callback)
end
function EventUtil.ContinueOnAddOnLoaded(name, callback)
  continueWhen("ADDON_LOADED", IsAddOnLoaded(name), callback,
    function(loaded) return string.lower(loaded) == string.lower(name) end)
end
-- Preserve the table identity while removing every keyed and sequence entry.
function table.wipe(value)
  for key in pairs(value) do value[key] = nil end
  return value
end
-- ClassicAPI registers it as a global too, which its addon reads back as `ClassicAPI.wipe`.
wipe = table.wipe

-- ClassicAPI's text-to-speech surface (voicechat/Tts.cpp), as it answers with no speech engine,
-- which this client has none of: no voices, and speech that plays nothing. Its settings page reads
-- the voice list, and the voice, rate and volume live in the ttsVoice/ttsSpeed/ttsVolume CVars.
C_VoiceChat = C_VoiceChat or {}
function C_VoiceChat.GetTtsVoices() return {} end
function C_VoiceChat.GetRemoteTtsVoices() return {} end
function C_VoiceChat.SpeakText(voiceID, text)
  if not tonumber(voiceID) or (type(text) ~= "string" and type(text) ~= "number") then
    error("Usage: C_VoiceChat.SpeakText(voiceID, text [, destination, rate, volume])", 2)
  end
end
function C_VoiceChat.StopSpeakingText() end
