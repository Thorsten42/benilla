-- The engine functions that read namespaces the rest of the install builds (C_Item, C_Spell,
-- C_Container, C_EquipmentSet): loaded last, before the ClassicAPI table is taken.
local bridge = ...
local strfind = string.find
function IsChatAFK() return UnitIsAFK("player") and true or false end
function IsChatDND() return UnitIsDND("player") and true or false end

-- Memory per addon is not measured here; the totals stay the VM's.
function UpdateAddOnMemoryUsage() end
function GetAddOnMemoryUsage(addon) return 0 end

-- Nobody's resurrection casts are tracked yet, so no unit has one incoming.
function UnitHasIncomingResurrection(unit) return false end

function C_AddOns.GetAddOnName(addon) return (GetAddOnInfo(addon)) end

function C_Item.GetItemIconByID(item)
  local _, _, _, _, icon = C_Item.GetItemInfoInstant(item)
  return icon
end

-- The explored overlays are not tracked yet: no map has a known unexplored piece.
C_MapExplorationInfo = C_MapExplorationInfo or {}
function C_MapExplorationInfo.GetUnexploredMapTextures(areaID) return {} end

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
