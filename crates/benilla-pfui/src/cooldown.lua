-- A ClassicAPI `Cooldown` frame here is 1.12's indicator model (CooldownFrameTemplate), its sweep
-- driven by the model's sequence time as 1.12's CooldownFrame_SetTimer drives it. Reverse runs
-- the sweep backwards (it starts clear and darkens), a cleared draw-bling flag hides the frame at
-- the end instead of the flash, a cleared draw-swipe flag keeps the timing and draws nothing, and
-- a pause holds the sweep. Colors and textures are kept for their getters.
BenillaCooldownMixin = {}
local M = BenillaCooldownMixin

-- The sweep's step, in place of the template's OnUpdateModel.
local function step()
  local self = this
  if self.benillaPaused then return end
  if self.stopping == 0 then
    local finished = (GetTime() - self.start) / self.duration
    if finished < 1 then
      if self:GetReverse() then finished = 1 - finished end
      self:SetSequenceTime(0, finished * 1000)
      return
    end
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
local setDrawSwipe = M.SetDrawSwipe
function M:SetDrawSwipe(draw)
  setDrawSwipe(self, draw)
  self:SetAlpha(draw == false and 0 or 1)
end
flag("DrawBling", true)
flag("HideCountdownNumbers", false)
flag("CountdownAbbrevThreshold", 120)
flag("CountdownMillisecondsThreshold", 0)
flag("MinimumCountdownDuration", 2000)
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
    self.benillaCountdown:SetPoint("CENTER", self, "CENTER", 0, 0)
  end
  return self.benillaCountdown
end
