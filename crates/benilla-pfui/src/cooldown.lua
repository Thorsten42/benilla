-- A ClassicAPI `Cooldown` frame here is 1.12's indicator model (CooldownFrameTemplate), stepped
-- from its OnUpdateModel. The swipe is four quarter textures over the frame (see `drawSwipe`) and
-- the model plays only the end's flash. Reverse darkens what has passed instead of what is left,
-- a cleared draw-bling flag hides the frame at the end instead of the flash, a cleared draw-swipe
-- flag keeps the timing and draws no swipe, and a pause holds it. Colors and textures are kept for
-- their getters.
BenillaCooldownMixin = {}
local M = BenillaCooldownMixin

-- The countdown numbers a ClassicAPI client writes over the sweep while `countdownForCooldowns` is
-- on (registered at 0, as the client registers it), the frame does not hide them and the cooldown
-- outlasts its minimum: retail's text for the time left, whole units rounded up, tenths below the
-- milliseconds threshold and m:ss below the abbreviation threshold, which counts from one minute
-- to one hour.
local known, value = pcall(GetCVar, "countdownForCooldowns")
if not known or value == nil then RegisterCVar("countdownForCooldowns", "0") end
local mod = math.mod or math.fmod
local function ceilDiv(n, d) return math.floor(n / d) + (mod(n, d) ~= 0 and 1 or 0) end
local function countdownText(ms, abbrevThreshold, millisecondsThreshold)
  if ms / 1000 < millisecondsThreshold then
    local tenths = ceilDiv(ms, 100)
    return math.floor(tenths / 10) .. "." .. mod(tenths, 10)
  end
  local s = ceilDiv(ms, 1000)
  if s >= 86400 then return ceilDiv(s, 86400) .. "d" end
  if s >= 3600 then return ceilDiv(s, 3600) .. "h" end
  if s >= 60 and s >= math.min(abbrevThreshold, 3600) then return ceilDiv(s, 60) .. "m" end
  if s >= 60 then return string.format("%d:%02d", math.floor(s / 60), mod(s, 60)) end
  return tostring(s)
end
-- The numbers for `ms` left, none for nil; the font string is made the first time it shows any.
local function countdown(self, ms)
  local text = ""
  if ms and ms > 0 and not self:GetHideCountdownNumbers()
    and self.duration * 1000 > self:GetMinimumCountdownDuration()
    and (tonumber(GetCVar("countdownForCooldowns") or 0) or 0) ~= 0 then
    local formatter = self.benillaCountdownFormatter
    if formatter then
      -- A formatter replaces the rules: it gets the seconds left, unrounded; an error shows none.
      local ok, formatted = pcall(formatter.FormatNumber, formatter, ms / 1000)
      text = ok and formatted or ""
    else
      text = countdownText(ms, self:GetCountdownAbbrevThreshold(),
        self:GetCountdownMillisecondsThreshold())
    end
  end
  if text == (self.benillaCountdownText or "") then return end
  self.benillaCountdownText = text
  self:GetCountdownFontString():SetText(text)
end

-- The flash fills the frame at any size, as a ClassicAPI client's does. 1.12's indicator model is
-- 36 units square at model scale 1, drawn up and right from the frame's bottom-left corner, so it
-- is scaled to the frame's larger side; the pane clips what overhangs the shorter one.
local function fit(self)
  local side = math.max(self:GetWidth() or 0, self:GetHeight() or 0)
  if side > 0 and side ~= self.benillaFitSide then
    self.benillaFitSide = side
    self:SetModelScale(side / 36)
  end
end

-- The swipe as a ClassicAPI client draws it (its frame/Cooldown.cpp, 4.3.4's fan): over the whole
-- frame, the dark part covers what is left of the cooldown, or with Reverse what has passed,
-- clockwise from twelve o'clock. The indicator model only clears clockwise, so it is held clear
-- and four quarter textures draw the swipe: each the model's own mask (`cooldown.blp`, a 60 %
-- black veil whose first texel column is clear), its coordinates turned so the clear edge lies
-- along the hand, as the model turns them.
local SWIPE_TEXTURE = "Interface\\Cooldown\\cooldown"
-- The quarters clockwise from twelve: the corner each is seated in, and its corners (UL, LL, UR,
-- LR) about the frame's centre in half-frame units, x right and y up.
local QUARTERS = {
  { "TOPRIGHT", "BOTTOMLEFT", { 0, 1, 0, 0, 1, 1, 1, 0 } },
  { "BOTTOMRIGHT", "TOPLEFT", { 0, 0, 0, -1, 1, 0, 1, -1 } },
  { "BOTTOMLEFT", "TOPRIGHT", { -1, 0, -1, -1, 0, 0, 0, -1 } },
  { "TOPLEFT", "BOTTOMRIGHT", { -1, 1, -1, 0, 0, 1, 0, 0 } },
}
-- The mask's u per half-frame across the hand: steep, so the clear column is a hairline and the
-- clamp holds each side.
local EDGE = 64
-- The model's time that leaves every quarter clear: each has turned its full quarter by then.
local CLEAR_MS = 999

local function quarterTexture(self, q)
  local swipe = self.benillaSwipe
  if not swipe then
    swipe = {}
    self.benillaSwipe = swipe
  end
  local tex = swipe[q]
  if not tex then
    tex = self:CreateTexture(nil, "ARTWORK")
    tex:SetTexture(SWIPE_TEXTURE)
    tex:SetPoint(QUARTERS[q][1], self, QUARTERS[q][1])
    tex:SetPoint(QUARTERS[q][2], self, "CENTER")
    swipe[q] = tex
  end
  return tex
end

local function hideSwipe(self)
  local swipe = self.benillaSwipe
  if not swipe then return end
  for q = 1, 4 do
    if swipe[q] then swipe[q]:Hide() end
  end
end

-- The dark span `[from, to]`, in turns clockwise from twelve, over quarter `q`.
local function drawQuarter(self, q, from, to)
  local a, b = (q - 1) * .25, q * .25
  local lo, hi = math.max(from, a), math.min(to, b)
  if hi <= lo then
    if self.benillaSwipe and self.benillaSwipe[q] then self.benillaSwipe[q]:Hide() end
    return
  end
  local tex = quarterTexture(self, q)
  tex:Show()
  if lo <= a and hi >= b then
    tex:SetTexCoord(.5, .5, .5, .5, .5, .5, .5, .5)
    return
  end
  -- One hand crosses the quarter: dark behind it (the span starts at the quarter's edge) or
  -- ahead of it. `sin(hand − angle)` of a corner is positive behind the hand.
  local hand, side
  if lo <= a then hand, side = hi, 1 else hand, side = lo, -1 end
  local s, c = math.sin(hand * 2 * math.pi), math.cos(hand * 2 * math.pi)
  local p = QUARTERS[q][3]
  local function u(i) return EDGE * side * (s * p[i + 1] - c * p[i]) end
  tex:SetTexCoord(u(1), .5, u(3), .5, u(5), .5, u(7), .5)
end

local function drawSwipe(self, finished)
  if not self:GetDrawSwipe() then
    hideSwipe(self)
    return
  end
  local from, to = finished, 1
  if self:GetReverse() then from, to = 0, finished end
  for q = 1, 4 do drawQuarter(self, q, from, to) end
end

-- The sweep's step, in place of the template's OnUpdateModel; the model plays the end's flash.
local function step()
  local self = this
  fit(self)
  if self.benillaPaused then return end
  if self.stopping == 0 then
    local elapsed = GetTime() - self.start
    local finished = elapsed / self.duration
    if finished < 1 then
      countdown(self, math.floor((self.duration - elapsed) * 1000))
      self:SetSequenceTime(0, CLEAR_MS)
      drawSwipe(self, math.max(finished, 0))
      return
    end
    countdown(self, nil)
    hideSwipe(self)
    if not self:GetDrawBling() then
      self:Hide()
      return
    end
    self.stopping = 1
    self:SetSequence(1)
    self:SetSequenceTime(1, 0)
  else
    self:AdvanceTime()
  end
end

local function flag(name, default)
  M["Set" .. name] = function(self, value) self["benilla" .. name] = value end
  M["Get" .. name] = function(self)
    local value = self["benilla" .. name]
    if value == nil then return default end
    return value
  end
end
flag("Reverse", false)
flag("DrawEdge", false)
flag("DrawSwipe", true)
flag("DrawBling", true)
flag("HideCountdownNumbers", false)
flag("CountdownAbbrevThreshold", 120)
flag("CountdownMillisecondsThreshold", 0)
flag("MinimumCountdownDuration", 2000)
-- The frame keeps the formatter itself, so a later change to its breakpoints changes the text.
function M:SetCountdownFormatter(formatter)
  if formatter ~= nil and (type(formatter) ~= "table" or type(formatter.FormatNumber) ~= "function") then
    error("Usage: " .. (self:GetName() or "<unnamed>") .. ":SetCountdownFormatter([formatter])", 2)
  end
  self.benillaCountdownFormatter = formatter
end
function M:GetCountdownFormatter() return self.benillaCountdownFormatter end
-- A ClassicAPI client has the type itself, and its CooldownFrame_SetTimer hands a frame that is one
-- to SetCooldown; for every other type the frame stays the Model it is built as.
function M:IsObjectType(kind)
  if kind == "Cooldown" then return true end
  return getmetatable(self).__index.IsObjectType(self, kind)
end
function M:SetSwipeColor() end
function M:SetEdgeColor() end
function M:SetSwipeTexture() end
function M:SetEdgeTexture() end
function M:SetBlingTexture() end

function M:SetCooldown(start, duration, modRate)
  if start and duration and start > 0 and duration > 0 then
    self.benillaTimes = {start, duration, modRate or 1}
    self.start, self.duration, self.stopping = start, duration, 0
    if not self.benillaStepping then
      self.benillaStepping = true
      self:SetScript("OnUpdateModel", step)
    end
    self:SetSequence(0)
    self:Show()
  else
    self:Clear()
  end
end
function M:SetCooldownDuration(duration, modRate) self:SetCooldown(GetTime(), duration, modRate) end
function M:SetCooldownFromExpirationTime(expiration, duration, modRate)
  self:SetCooldown(expiration - duration, duration, modRate)
end
function M:Clear()
  self.benillaTimes = nil
  countdown(self, nil)
  hideSwipe(self)
  self:Hide()
end
function M:GetCooldownTimes()
  local times = self.benillaTimes
  if not times then return 0, 0 end
  return times[1] * 1000, times[2] * 1000
end
function M:GetCooldownDuration()
  local _, duration = self:GetCooldownTimes()
  return duration
end
function M:GetCooldownDisplayDuration()
  local times = self.benillaTimes
  return times and times[2] * 1000 / times[3] or 0
end
function M:SetPaused(paused) self.benillaPaused = paused and true or false end
function M:IsPaused() return self.benillaPaused == true end
function M:Pause() self:SetPaused(true) end
function M:Resume() self:SetPaused(false) end
function M:GetCountdownFontString()
  if not self.benillaCountdown then
    self.benillaCountdown = self:CreateFontString(nil, "OVERLAY")
    self.benillaCountdown:SetFont("Fonts\\FRIZQT__.TTF", 16, "OUTLINE")
    self.benillaCountdown:SetPoint("CENTER", self, "CENTER", 0, -1)
  end
  return self.benillaCountdown
end
