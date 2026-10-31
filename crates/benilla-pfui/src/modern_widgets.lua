-- Widget methods a ClassicAPI client adds to 1.12's, which its Lua addon and pfUI call. A flag
-- this engine does not draw (clipping, alpha gradients, masks) is kept, so its getter answers.
local tables = ...
-- Each table takes a method only where it has none: a native one stays.
local function adding(target)
  return setmetatable({}, {
    __index = target,
    __newindex = function(_, name, func)
      if target[name] == nil then rawset(target, name, func) end
    end,
  })
end
local frame, button = adding(tables.frame), adding(tables.button)
local editbox, slider = adding(tables.editbox), adding(tables.slider)
local statusbar, texture = adding(tables.statusbar), adding(tables.texture)
local fontstring = adding(tables.fontstring)
local regions = {frame, texture, fontstring, adding(tables.title), adding(tables.region)}

local state = setmetatable({}, {__mode = "k"})
local function flags(object)
  local own = state[object]
  if not own then own = {}; state[object] = own end
  return own
end
local function setter(name)
  return function(self, value) flags(self)[name] = value end
end
local function getter(name, default)
  return function(self)
    local value = flags(self)[name]
    if value == nil then return default end
    return value
  end
end

-- Regions: the rect, one point by name, and the cursor over it.
local function rect(self)
  local left, bottom = self:GetLeft(), self:GetBottom()
  if not left or not bottom then return nil end
  return left, bottom, self:GetWidth(), self:GetHeight()
end
local function points(self)
  local list = {}
  for i = 1, self:GetNumPoints() do
    local point, relative, relativePoint, x, y = self:GetPoint(i)
    list[i] = {point, relative, relativePoint, x, y}
  end
  return list
end
local region = {}
region.GetRect = rect
function region:GetScaledRect()
  local left, bottom, width, height = rect(self)
  if not left then return nil end
  local scale = self:GetEffectiveScale()
  return left * scale, bottom * scale, width * scale, height * scale
end
function region:IsRectValid() return self:GetLeft() ~= nil end
function region:GetPointByName(name)
  for _, p in ipairs(points(self)) do
    if p[1] == name then return p[1], p[2], p[3], p[4], p[5] end
  end
end
function region:ClearPoint(name)
  local kept = {}
  for _, p in ipairs(points(self)) do
    if p[1] ~= name then tinsert(kept, p) end
  end
  self:ClearAllPoints()
  for _, p in ipairs(kept) do self:SetPoint(p[1], p[2], p[3], p[4], p[5]) end
end
function region:AdjustPointsOffset(dx, dy)
  local list = points(self)
  self:ClearAllPoints()
  for _, p in ipairs(list) do self:SetPoint(p[1], p[2], p[3], (p[4] or 0) + dx, (p[5] or 0) + dy) end
end
function region:GetDebugName()
  return self:GetName() or tostring(self)
end
function region:IsMouseOver(top, bottom, left, right)
  local x0, y0, width, height = rect(self)
  if not x0 then return false end
  local scale = self:GetEffectiveScale()
  local x, y = GetCursorPosition()
  x, y = x / scale, y / scale
  return x >= x0 + (left or 0) and x <= x0 + width + (right or 0)
    and y >= y0 + (bottom or 0) and y <= y0 + height + (top or 0)
end
for _, methods in ipairs(regions) do
  for name, func in pairs(region) do methods[name] = func end
end

-- Frames.
function frame:SetClampRectInsets(left, right, top, bottom)
  flags(self).clampInsets = {left or 0, right or 0, top or 0, bottom or 0}
end
function frame:GetClampRectInsets()
  local insets = flags(self).clampInsets or {0, 0, 0, 0}
  return insets[1], insets[2], insets[3], insets[4]
end
frame.SetClipsChildren = setter("clips")
frame.DoesClipChildren = getter("clips", false)
function frame:SetResizeBounds(minWidth, minHeight, maxWidth, maxHeight)
  self:SetMinResize(minWidth or 0, minHeight or 0)
  if maxWidth then self:SetMaxResize(maxWidth, maxHeight or maxWidth) end
end
frame.GetBoundsRect = rect
function frame:GetEffectiveAlpha()
  local alpha, at = 1, self
  while at do
    alpha = alpha * at:GetAlpha()
    at = at:GetParent()
  end
  return alpha
end
function frame:SetAlphaGradient() end
function frame:ClearAlphaGradient() end
function frame:HasAlphaGradient() return false end
function frame:DesaturateHierarchy(amount)
  for _, region in ipairs({self:GetRegions()}) do
    if region.SetDesaturated then region:SetDesaturated(amount and amount > 0) end
  end
end
-- Click and motion enable one mouse flag in 1.12: either keeps it on.
local function mouse(self, which, on)
  local own = flags(self)
  own[which] = on and true or false
  local click = own.click
  local motion = own.motion
  if click == nil then click = self:IsMouseEnabled() and true or false end
  if motion == nil then motion = click end
  self:EnableMouse(click or motion)
end
function frame:SetMouseClickEnabled(on) mouse(self, "click", on) end
function frame:SetMouseMotionEnabled(on) mouse(self, "motion", on) end
function frame:IsMouseClickEnabled() return self:IsMouseEnabled() and true or false end
function frame:IsMouseMotionEnabled() return self:IsMouseEnabled() and true or false end
function frame:IsMouseMotionFocus() return GetMouseFocus() == self end
frame.SetPropagateMouseMotion = setter("propagateMotion")
frame.GetPropagateMouseMotion = getter("propagateMotion", false)
frame.SetPropagateMouseClicks = setter("propagateClicks")
frame.GetPropagateMouseClicks = getter("propagateClicks", false)
frame.SetPropagateKeyboardInput = setter("propagateKeys")
frame.GetPropagateKeyboardInput = getter("propagateKeys", false)
frame.SetDontSavePosition = setter("dontSave")
frame.GetDontSavePosition = getter("dontSave", false)

-- Buttons.
function button:SetEnabled(on)
  if on then self:Enable() else self:Disable() end
end
button.RegisterForMouse = setter("mouseButtons")
button.SetMotionScriptsWhileDisabled = setter("motionWhileDisabled")
button.GetMotionScriptsWhileDisabled = getter("motionWhileDisabled", false)
button.SetNormalFontObject = button.SetTextFontObject
button.GetNormalFontObject = button.GetTextFontObject

-- Edit boxes and sliders: enabled is a flag; a disabled edit box drops its focus.
for _, methods in ipairs({editbox, slider}) do
  function methods:Enable() flags(self).disabled = false end
  function methods:Disable()
    flags(self).disabled = true
    if self.ClearFocus then self:ClearFocus() end
  end
  function methods:IsEnabled() return not flags(self).disabled end
  function methods:SetEnabled(on) if on then self:Enable() else self:Disable() end end
end
function editbox:ClearHighlightText() self:HighlightText(0, 0) end

-- Font strings.
fontstring.GetUnboundedStringWidth = fontstring.GetStringWidth
function fontstring:GetStringHeight() return self:GetHeight() end
function fontstring:IsTruncated()
  local width = self:GetWidth()
  return width ~= nil and width > 0 and self:GetStringWidth() > width + 0.5
end
function fontstring:SetTextToFit(text)
  self:SetText(text)
  self:SetWidth(self:GetStringWidth())
end

-- Status bars.
statusbar.SetReverseFill = setter("reverseFill")
statusbar.GetReverseFill = getter("reverseFill", false)
statusbar.SetFillStyle = setter("fillStyle")
statusbar.GetFillStyle = getter("fillStyle", "STANDARD")
statusbar.SetRotatesTexture = setter("rotatesTexture")
statusbar.GetRotatesTexture = getter("rotatesTexture", false)
function statusbar:SetColorFill(r, g, b, a) self:SetStatusBarColor(r, g, b, a) end

-- Textures: desaturation as 1.12's flag, masks kept and not drawn.
function texture:SetDesaturation(amount)
  self:SetDesaturated(amount ~= nil and amount > 0)
  flags(self).desaturation = amount or 0
end
texture.GetDesaturation = getter("desaturation", 0)
texture.SetMask = setter("mask")
function texture:AddMaskTexture(mask)
  local own = flags(self)
  own.masks = own.masks or {}
  tinsert(own.masks, mask)
end
function texture:RemoveMaskTexture(mask)
  local list = flags(self).masks or {}
  for i = table.getn(list), 1, -1 do
    if list[i] == mask then table.remove(list, i) end
  end
end
function texture:GetNumMaskTextures() return table.getn(flags(self).masks or {}) end
function texture:GetMaskTexture(index) return (flags(self).masks or {})[index] end
function frame:CreateMaskTexture(name, layer)
  local mask = self:CreateTexture(name, layer or "ARTWORK")
  mask:Hide()
  return mask
end

-- `SetPoint("point")`: the parent's same point, no offset.
for _, methods in ipairs({tables.frame, tables.texture, tables.fontstring}) do
  local setPoint = methods.SetPoint
  methods.SetPoint = function(self, ...)
    if select("#", ...) == 1 then
      local point = ...
      return setPoint(self, point, self:GetParent(), point, 0, 0)
    end
    return setPoint(self, ...)
  end
end

-- OnEnable and OnDisable: script slots on a widget that can be enabled, run when Enable or
-- Disable changes its state.
local stateScripts = setmetatable({}, {__mode = "k"})
local stateSlots = {onenable = "OnEnable", ondisable = "OnDisable"}
local function stateSlot(self, kind)
  return type(kind) == "string" and self.Enable ~= nil and stateSlots[string.lower(kind)]
end
local base = tables.frame
local setScript, getScript, hasScript = base.SetScript, base.GetScript, base.HasScript
base.SetScript = function(self, kind, handler)
  local slot = stateSlot(self, kind)
  if not slot then return setScript(self, kind, handler) end
  stateScripts[self] = stateScripts[self] or {}
  stateScripts[self][slot] = handler
end
base.GetScript = function(self, kind)
  local slot = stateSlot(self, kind)
  if not slot then return getScript(self, kind) end
  return (stateScripts[self] or {})[slot]
end
base.HasScript = function(self, kind)
  if stateSlot(self, kind) then return 1 end
  return hasScript(self, kind)
end
local function fireState(self, slot)
  local handler = (stateScripts[self] or {})[slot]
  if not handler then return end
  local before = this
  this = self
  handler(self)
  this = before
end
for _, methods in ipairs({tables.button, tables.editbox, tables.slider}) do
  local enable, disable, isEnabled = methods.Enable, methods.Disable, methods.IsEnabled
  if enable and disable and isEnabled then
    methods.Enable = function(self)
      local was = isEnabled(self)
      enable(self)
      if not was then fireState(self, "OnEnable") end
    end
    methods.Disable = function(self)
      local was = isEnabled(self)
      disable(self)
      if was then fireState(self, "OnDisable") end
    end
  end
end
