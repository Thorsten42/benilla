local register = ...
-- ClassicAPI's CVars, each at the default its C++ registers (src/**/*.cpp, v1.23.9), so its
-- Settings window and the addons that read them find every one. They are the engine's, as
-- ClassicAPI's are, so they leave the scripts' RegisterCVar budget of 30 to the addons; a CVar the
-- client already has keeps its own. Of these, benilla acts on countdownForCooldowns (cooldown.lua), the three
-- autoComplete settings (autocomplete.lua) and the bag-sort ones (bagsort.lua); the others are
-- kept with no effect yet: soft targeting and target priority, auto-target and key-down action
-- buttons, the nameplate filters, the damage meter and combat log file, the frame-rate caps and
-- the rest of ClassicAPI's own features.
local cvars = {
  -- action/UseKeyDown.cpp, ui/ShakeStrength.cpp
  ActionButtonUseKeyDown = "0", ShakeStrengthUI = "1.000000",
  -- target/SoftUnit.cpp, Reason.cpp, SoftTargetIcon.cpp, SoftInteract.cpp
  SoftTargetEnemy = "0", SoftTargetEnemyArc = "2", SoftTargetEnemyRange = "45",
  SoftTargetFriend = "0", SoftTargetFriendArc = "2", SoftTargetFriendRange = "45",
  SoftTargetForce = "1", SoftTargetMatchLocked = "1", SoftTargetWithLocked = "1",
  SoftTargetIconEnemy = "0", SoftTargetIconFriend = "0", SoftTargetIconGameObject = "0",
  SoftTargetIconInteract = "1", SoftTargetLowPriorityIcons = "0", SoftTargetNameplateSize = "19",
  SoftTargetWorldtextFarDist = "40", SoftTargetWorldtextNearDist = "4",
  SoftTargetWorldtextNearScale = "1", SoftTargetWorldtextSize = "32",
  SoftTargetInteract = "0", SoftTargetInteractArc = "0", SoftTargetInteractRange = "10",
  SoftTargetInteractRangeIsHard = "0",
  -- nameplate/Filter.cpp, NameColor.cpp; frame/Alpha.cpp
  SoftTargetNameplateFriend = "0", SoftTargetNameplateInteract = "0",
  nameplateShowEnemyGuardians = "1", nameplateShowEnemyMinions = "1",
  nameplateShowEnemyPets = "1", nameplateShowEnemyTotems = "1", nameplateShowFriendlyNpcs = "1",
  nameplateShowFriendlyPlayerGuardians = "1", nameplateShowFriendlyPlayerMinions = "1",
  nameplateShowFriendlyPlayerPets = "1", nameplateShowFriendlyPlayerTotems = "0",
  nameplateShowFriendlyPlayers = "1", nameplateUseClassColorForFriendlyPlayerUnitNames = "0",
  nameplateNotSelectedAlpha = "0.5", nameplateSelectedAlpha = "1",
  -- target/SoftCast.cpp, Reason.cpp, AutoTarget.cpp, Tab.cpp, Priority.cpp
  TargetAutoEnemy = "1", TargetAutoFriend = "0", TargetAutoLock = "1", TargetEnemyAttacker = "1",
  TargetNearestUseNew = "0", TargetPriorityAllowAnyOnScreen = "0",
  TargetPriorityAutoTargetIgnoreWindow = "500", TargetPriorityCombatLock = "1",
  TargetPriorityCombatLockContextualRelaxation = "1", TargetPriorityContinueFromManualTarget = "1",
  TargetPriorityPvp = "1", TargetPriorityPvpLock = "0",
  -- combatlog/File.cpp, Movement.cpp, History.cpp; damagemeter/Live.cpp
  advancedCombatLogging = "0", advancedCombatLoggingMovement = "0", combatLogRetentionTime = "300",
  damageMeterEnabled = "0", damageMeterResetOnNewInstance = "0",
  -- container/SortBags.cpp
  sortBagsRightToLeft = "0", backpackAutosortDisabled = "0", bankAutosortDisabled = "0",
  -- autocomplete/Names.cpp, localization/Numbers.cpp, equipmentset/Tooltip.cpp, cursor/Extended.cpp
  autoCompleteResortNamesOnRecency = "1", autoCompleteUseContext = "1",
  autoCompleteWhenEditingFromCenter = "1", breakUpLargeNumbers = "1",
  dontShowEquipmentSetsOnItems = "0", expandedCursors = "0",
  -- spell/AutoPrepare.cpp, Placement.cpp; combat/TargetChange.cpp; frame/Cooldown.cpp
  autoDismount = "0", autoStand = "0", autoUnshift = "0", spellClampingEnable = "1",
  spellClampingMinRange = "0.0f", stopAutoAttackOnTargetChange = "0", countdownForCooldowns = "0",
  -- gx/MaxFps.cpp, MaxFpsLoading.cpp; profile/Cpu.cpp
  maxFPS = "120", maxFPSBk = "30", maxFPSLoading = "10", useMaxFPS = "0", useMaxFPSBk = "0",
  scriptProfile = "0",
}
for name, value in pairs(cvars) do register(name, value) end
