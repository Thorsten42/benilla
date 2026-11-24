-- ClassicAPI 1.23's ModelScene frame type and C_MountJournal, as far as this client serves them.
--
-- The ModelScene intrinsic is a frame (lib.rs INTRINSICS) whose scene keeps its flags and draws
-- nothing: actors and cameras come from C_ModelInfo's scene data, which this client does not
-- have, so ModelSceneMixin:SetFromModelSceneID and TransitionToModelSceneID return before they
-- create any, and the dressing room's mount mode, its one user, never opens (see
-- GetMountInfoExtraByID). The frame, its control frame and its scripts work as they do.
BenillaModelSceneMixin = {}
-- A ClassicAPI client has the type itself; here the frame is built as a Frame.
function BenillaModelSceneMixin:GetObjectType()
  return "ModelScene"
end
function BenillaModelSceneMixin:IsObjectType(kind)
  if kind == "ModelScene" then return true end
  return getmetatable(self).__index.IsObjectType(self, kind)
end
function BenillaModelSceneMixin:SetAllowOverlappedModels(allow)
  self.benillaAllowOverlappedModels = allow and true or false
end
function BenillaModelSceneMixin:GetAllowOverlappedModels()
  return self.benillaAllowOverlappedModels or false
end

-- ClassicAPI's C_MountJournal (mount/Journal.cpp): 1.12 has no Mount table, so a mount's ID is
-- its spell ID, and a spell is a mount when one of its effects applies SPELL_AURA_MOUNTED (78),
-- read from Spell.dbc whether known or not. An item is a mount when its use spell is one; on
-- Turtle every mount item teaches through 46499, which is not, so only the spell resolves.
local SPELL_AURA_MOUNTED = 78
C_MountJournal = C_MountJournal or {}
function C_MountJournal.GetMountFromSpell(spellID)
  local id = tonumber(spellID)
  local effects = id and C_Spell.GetSpellEffectInfo(id)
  if not effects then return nil end
  for _, effect in ipairs(effects) do
    if effect.auraName == SPELL_AURA_MOUNTED then return id end
  end
  return nil
end
function C_MountJournal.GetMountFromItem(itemID)
  local _, spellID = C_Item.GetItemSpell(tonumber(itemID) or itemID)
  return spellID and C_MountJournal.GetMountFromSpell(spellID) or nil
end
-- ClassicAPI reads the mount creature's display from the client's creature cache, nil (and a
-- cache load) while it is not there. This client's UI cannot read that cache, so every mount
-- answers as not cached and DressUpMount waits for a load that does not come.
function C_MountJournal.GetMountInfoExtraByID(mountID)
  return nil
end
