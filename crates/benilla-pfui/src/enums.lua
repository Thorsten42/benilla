-- ClassicAPI's Enum tables are classicapi_enums.lua's.

-- Slot IDs match the host's 1.12 inventory snapshots (ammo is slot zero).
local slots = { 'AMMO', 'HEAD', 'NECK', 'SHOULDER', 'BODY', 'CHEST', 'WAIST',
  'LEGS', 'FEET', 'WRIST', 'HAND', 'FINGER1', 'FINGER2', 'TRINKET1', 'TRINKET2',
  'BACK', 'MAINHAND', 'OFFHAND', 'RANGED', 'TABARD' }
for index, name in ipairs(slots) do _G['INVSLOT_' .. name] = index - 1 end
INVSLOT_FIRST_EQUIPPED, INVSLOT_LAST_EQUIPPED = 1, 19
INVSLOTS_EQUIPABLE_IN_COMBAT = { [16] = true, [17] = true, [18] = true }
HEARTHSTONE_ITEM_ID = 6948
FIRE_TOTEM_SLOT, EARTH_TOTEM_SLOT, WATER_TOTEM_SLOT, AIR_TOTEM_SLOT, MAX_TOTEMS = 1, 2, 3, 4, 4
