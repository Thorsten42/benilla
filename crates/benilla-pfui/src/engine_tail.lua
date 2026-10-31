-- The engine functions that read namespaces the rest of the install builds (C_Item, C_Spell,
-- C_Container, C_EquipmentSet): loaded last, before the ClassicAPI table is taken.
local bridge = ...
local strfind = string.find
function IsChatAFK() return UnitIsAFK("player") and true or false end
function IsChatDND() return UnitIsDND("player") and true or false end

-- Memory per addon is not measured here; the totals stay the VM's.
function UpdateAddOnMemoryUsage() end
function GetAddOnMemoryUsage(addon) return 0 end

function C_AddOns.GetAddOnName(addon) return (GetAddOnInfo(addon)) end

function C_Item.GetItemIconByID(item)
  local _, _, _, _, icon = C_Item.GetItemInfoInstant(item)
  return icon
end

-- The bag search: an item matches when the text is part of its name, its subclass, its equip
-- slot's name or the name of an equipment set holding it.
local search = ""
local function setsHolding(bag, slot)
  local names = {}
  if not (C_EquipmentSet and C_EquipmentSet.GetEquipmentSetIDs) then return names end
  for _, id in ipairs(C_EquipmentSet.GetEquipmentSetIDs() or {}) do
    for _, location in pairs(C_EquipmentSet.GetItemLocations(id) or {}) do
      local _, _, inBags, setSlot, setBag = EquipmentManager_UnpackLocation(location)
      if inBags and setSlot == slot and setBag == bag then
        tinsert(names, (C_EquipmentSet.GetEquipmentSetInfo(id)))
        break
      end
    end
  end
  return names
end
function C_Container.GetContainerItemEquipmentSetInfo(bag, slot)
  local names = setsHolding(bag, slot)
  if table.getn(names) == 0 then return false, nil end
  return true, table.concat(names, PLAYER_LIST_DELIMITER or ", ")
end
local function matches(bag, slot, link)
  if search == "" then return true end
  local _, _, name = strfind(link or "", "%[(.-)%]")
  local itemID = C_Container.GetContainerItemID(bag, slot)
  local _, _, subType, equipLoc = C_Item.GetItemInfoInstant(itemID or 0)
  for _, text in ipairs({name or "", subType or "", equipLoc and getglobal(equipLoc) or ""}) do
    if strfind(string.lower(text), search, 1, true) then return true end
  end
  for _, set in ipairs(setsHolding(bag, slot)) do
    if strfind(string.lower(set), search, 1, true) then return true end
  end
  return false
end
function C_Container.SetItemSearch(text)
  text = string.lower(text or "")
  if text == search then return end
  search = text
  bridge.fire("INVENTORY_SEARCH_UPDATE")
end
function C_Container.GetContainerItemInfo(bag, slot)
  local texture, count, locked, quality, readable = GetContainerItemInfo(bag, slot)
  if not texture then return nil end
  local link = GetContainerItemLink(bag, slot)
  return {
    iconFileID = texture, stackCount = count, isLocked = locked and true or false,
    quality = quality, isReadable = readable and true or false, hasLoot = false,
    hyperlink = link, isFiltered = not matches(bag, slot, link), hasNoValue = false,
    itemID = C_Container.GetContainerItemID(bag, slot), isBound = false,
  }
end

-- The ClassicAPI addon this layer's engine half is written for, v1.18.5: once it has loaded, the
-- version an addon's load gate reads is that one. Without it the layer stands alone at 1.15.10.
local versionFrame = CreateFrame("Frame")
versionFrame:RegisterEvent("ADDON_LOADED")
versionFrame:SetScript("OnEvent", function()
  if arg1 == "!!!ClassicAPI" then
    CLASSIC_API_VERSION = 11805
    BenillaClassicAPICompatibility.apiVersion = "1.18.5"
    versionFrame:UnregisterEvent("ADDON_LOADED")
  end
end)

-- The paper doll slot for an inventory slot number, the reverse of GetInventorySlotInfo.
local slotNames = {
  "HeadSlot", "NeckSlot", "ShoulderSlot", "ShirtSlot", "ChestSlot", "WaistSlot", "LegsSlot",
  "FeetSlot", "WristSlot", "HandsSlot", "Finger0Slot", "Finger1Slot", "Trinket0Slot",
  "Trinket1Slot", "BackSlot", "MainHandSlot", "SecondaryHandSlot", "RangedSlot", "TabardSlot",
  "Bag0Slot", "Bag1Slot", "Bag2Slot", "Bag3Slot",
}
slotNames[0] = "AmmoSlot"
C_PaperDollInfo = C_PaperDollInfo or {}
function C_PaperDollInfo.GetInventorySlotInfoForInvSlot(invSlot)
  if type(invSlot) ~= "number" then
    error("Usage: C_PaperDollInfo.GetInventorySlotInfoForInvSlot(invSlot)", 2)
  end
  local name = slotNames[invSlot]
  if not name then return end
  local id, texture = GetInventorySlotInfo(name)
  return id, texture, invSlot == 18, name
end

-- Loss of control: the player's harmful auras whose spell applies a control aura, by its aura
-- type. A per-frame diff raises LOSS_OF_CONTROL_ADDED for each new effect and one
-- LOSS_OF_CONTROL_UPDATE when the set changed. School lockouts are not seen yet.
local controlAuras = {
  [2] = "POSSESS", [5] = "CONFUSE", [6] = "CHARM", [7] = "FEAR", [12] = "STUN", [25] = "PACIFY",
  [26] = "ROOT", [27] = "SILENCE", [60] = "PACIFYSILENCE", [67] = "DISARM",
}
local controlOf = {}
local function controlType(spellID)
  local known = controlOf[spellID]
  if known ~= nil then return known or nil end
  local found = false
  local effects = C_Spell and C_Spell.GetSpellEffectInfo and C_Spell.GetSpellEffectInfo(spellID)
  for _, effect in ipairs(effects or {}) do
    if controlAuras[effect.auraName] then found = controlAuras[effect.auraName] break end
  end
  controlOf[spellID] = found
  return found or nil
end
local active = {}
local function scan()
  local now, list = GetTime(), {}
  for i = 1, 16 do
    local name, icon, _, _, duration, expiration, _, _, _, spellID =
      C_UnitAuras.UnitAura("player", i, "HARMFUL")
    if not name then break end
    local locType = spellID and controlType(spellID)
    if locType then
      local timed = duration and duration > 0 and expiration and expiration > 0
      tinsert(list, {
        locType = locType, spellID = spellID, displayText = name, iconTexture = icon or "",
        startTime = timed and expiration - duration or nil,
        timeRemaining = timed and math.max(0, expiration - now) or nil,
        duration = timed and duration or nil, lockoutSchool = 0, priority = 0, displayType = 2,
      })
    end
  end
  return list
end
local function key(entry) return entry.locType .. ":" .. entry.spellID end
local lossFrame = CreateFrame("Frame")
lossFrame:SetScript("OnUpdate", function()
  if not UnitExists("player") then return end
  local list = scan()
  local before = {}
  for _, entry in ipairs(active) do before[key(entry)] = true end
  local changed = table.getn(list) ~= table.getn(active)
  active = list
  for index, entry in ipairs(list) do
    if not before[key(entry)] then
      changed = true
      bridge.fire("LOSS_OF_CONTROL_ADDED", index)
    end
  end
  if changed then bridge.fire("LOSS_OF_CONTROL_UPDATE", "player") end
end)
C_LossOfControl = C_LossOfControl or {}
function C_LossOfControl.GetActiveLossOfControlDataCount() return table.getn(active) end
function C_LossOfControl.GetActiveLossOfControlData(index)
  local entry = active[index]
  if not entry then return nil end
  local copy = {}
  for field, value in pairs(entry) do copy[field] = value end
  if copy.startTime then copy.timeRemaining = math.max(0, copy.startTime + copy.duration - GetTime()) end
  return copy
end

-- CopyToClipboard: the client writes the text once this frame's scripts are done.
function CopyToClipboard(text, removeMarkup)
  text = tostring(text or "")
  if removeMarkup then
    text = string.gsub(text, "|c%x%x%x%x%x%x%x%x", "")
    text = string.gsub(text, "|r", "")
    text = string.gsub(text, "|H.-|h(.-)|h", "%1")
    text = string.gsub(text, "|T.-|t", "")
    text = string.gsub(text, "|A.-|a", "")
    text = string.gsub(text, "|n", "\n")
    text = string.gsub(text, "||", "|")
  end
  bridge.clipboard(text)
  return string.len(text)
end
