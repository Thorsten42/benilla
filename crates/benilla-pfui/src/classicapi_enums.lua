-- Every Enum table ClassicAPI's C++ registers (RegisterIntegerEnum, v1.23.9), with its values, in
-- one place: the layer's modules and the ClassicAPI addon read these. Each line names the file that
-- registers it. Loaded before any other chunk of the layer, which only adds tables of its own.
Enum = Enum or {}
local enums = {
  -- action/Find.cpp
  ActionBarSet = { None = 0, Mkb = 1, Gamepad = 2, All = 3 },
  -- addons/EnableState.cpp
  AddOnEnableState = { None = 0, Some = 1, All = 2 },
  -- addons/Info.cpp
  AddOnSecurityStatus = { Secure = 0, Insecure = 1, Banned = 2, NotAvailable = 3 },
  -- autocomplete/Names.cpp
  AutoCompleteEntryFlag = {
    InGroup = 1, InGuild = 2, Friend = 4, Bnet = 8, InteractedWith = 16, Online = 32, InAOI = 64,
    AccountCharacter = 128, RecentPlayer = 256,
  },
  -- autocomplete/Names.cpp
  AutoCompletePriority = {
    Other = 0, Interacted = 1, InGroup = 2, Guild = 3, Friend = 4, AccountCharacter = 5,
    AccountCharacterSameRealm = 6,
  },
  -- container/BagIndex.cpp
  BagIndex = {
    Keyring = -2, Bank = -1, Backpack = 0, Bag_1 = 1, Bag_2 = 2, Bag_3 = 3, Bag_4 = 4,
    BankBag_1 = 5, BankBag_2 = 6, BankBag_3 = 7, BankBag_4 = 8, BankBag_5 = 9, BankBag_6 = 10,
  },
  -- container/SlotFlags.cpp
  BagSlotFlags = {
    DisableAutoSort = 1, ClassEquipment = 2, ClassConsumables = 4, ClassProfessionGoods = 8,
    ClassJunk = 16, ClassQuestItems = 32, ExcludeJunkSell = 64, ClassReagents = 128,
    ExpansionCurrent = 256, ExpansionLegacy = 512,
  },
  -- bindings/Api.cpp
  BindingSet = { Default = 0, Account = 1, Character = 2, Current = 3 },
  -- encoding/Compress.cpp
  CompressionLevel = { Default = 0, OptimizeForSpeed = 1, OptimizeForSize = 2 },
  -- encoding/Compress.cpp
  CompressionMethod = { Deflate = 0, Zlib = 1, Gzip = 2 },
  -- console/Commands.cpp
  ConsoleCategory = {
    Debug = 0, Graphics = 1, Console = 2, Combat = 3, Game = 4, Default = 5, Net = 6, Sound = 7,
    Gm = 8, Reveal = 9, None = 10,
  },
  -- console/Commands.cpp
  ConsoleCommandType = { Cvar = 0, Command = 1, Macro = 2, Script = 3 },
  -- damagemeter/Enums.cpp
  DamageMeterCombineSessionType = { None = 0, ChallengeMode = 1, Arena = 2, ArenaMultiRound = 3 },
  -- damagemeter/Enums.cpp
  DamageMeterNumbers = { Minimal = 0, Compact = 1, Complete = 2 },
  -- damagemeter/Enums.cpp
  DamageMeterOverrideType = {
    Ignore = 0, AllowFriendlyFire = 1, RedirectSourceToOwner = 2, RedirectSourceToAuraCaster = 3,
    IgnoreForAbsorbSpell = 4,
  },
  -- damagemeter/Enums.cpp
  DamageMeterSessionType = { Overall = 0, Current = 1, Expired = 2 },
  -- damagemeter/Enums.cpp
  DamageMeterSourceDisplayType = { None = 0, Ally = 1, Enemy = 2 },
  -- damagemeter/Enums.cpp
  DamageMeterSpellDetailsDisplayType = {
    SpellCasted = 0, UnitSpecificSpellCasted = 1, SpellAffected = 2, Deaths = 3,
    EnemyDamageTaken = 4,
  },
  -- damagemeter/Enums.cpp
  DamageMeterStorageType = {
    Damage = 0, HealingAndAbsorbs = 1, Absorbs = 2, Interrupts = 3, Dispels = 4, DamageTaken = 5,
    AvoidableDamageTaken = 6, Deaths = 7, EnemyDamageTaken = 8,
  },
  -- damagemeter/Enums.cpp
  DamageMeterStyle = { Default = 0, Thin = 1, Bordered = 2, FullBackground = 3 },
  -- damagemeter/Enums.cpp
  DamageMeterType = {
    DamageDone = 0, Dps = 1, HealingDone = 2, Hps = 3, Absorbs = 4, Interrupts = 5, Dispels = 6,
    DamageTaken = 7, AvoidableDamageTaken = 8, Deaths = 9, EnemyDamageTaken = 10,
  },
  -- damagemeter/Enums.cpp
  DamageMeterVisibility = { Always = 0, InCombat = 1, Hidden = 2, InGroup = 3 },
  -- duration/TextBinding.cpp
  DurationTextBindingProperty = {
    RemainingDuration = 0, RemainingPercent = 1, ElapsedDuration = 2, ElapsedPercent = 3,
    TotalDuration = 4, StartTime = 5, EndTime = 6,
  },
  -- duration/Object.cpp
  DurationTimeModifier = { RealTime = 0, BaseTime = 1 },
  -- taxi/Map.cpp
  FlightPathFaction = { Neutral = 0, Horde = 1, Alliance = 2 },
  -- taxi/Map.cpp
  FlightPathState = { Current = 0, Reachable = 1, Unreachable = 2 },
  -- gossip/Info.cpp
  GossipOptionStatus = { Available = 0, Unavailable = 1, Locked = 2, AlreadyComplete = 3 },
  -- editbox/InputContext.cpp
  InputContext = { None = 0, Keyboard = 1, Mouse = 2, GamePad = 3 },
  -- item/TypeInfo.cpp
  InventoryType = {
    IndexNonEquipType = 0, IndexHeadType = 1, IndexNeckType = 2, IndexShoulderType = 3,
    IndexBodyType = 4, IndexChestType = 5, IndexWaistType = 6, IndexLegsType = 7,
    IndexFeetType = 8, IndexWristType = 9, IndexHandType = 10, IndexFingerType = 11,
    IndexTrinketType = 12, IndexWeaponType = 13, IndexShieldType = 14, IndexRangedType = 15,
    IndexCloakType = 16, Index2HweaponType = 17, IndexBagType = 18, IndexTabardType = 19,
    IndexRobeType = 20, IndexWeaponmainhandType = 21, IndexWeaponoffhandType = 22,
    IndexHoldableType = 23, IndexAmmoType = 24, IndexThrownType = 25, IndexRangedrightType = 26,
    IndexQuiverType = 27, IndexRelicType = 28, IndexProfessionToolType = 29,
    IndexProfessionGearType = 30, IndexEquipablespellOffensiveType = 31,
    IndexEquipablespellUtilityType = 32, IndexEquipablespellDefensiveType = 33,
    IndexEquipablespellWeaponType = 34,
  },
  -- item/TypeInfo.cpp
  ItemClass = {
    Consumable = 0, Container = 1, Weapon = 2, Gem = 3, Armor = 4, Reagent = 5, Projectile = 6,
    Tradegoods = 7, ItemEnhancement = 8, Recipe = 9, CurrencyTokenObsolete = 10, Quiver = 11,
    Questitem = 12, Key = 13, PermanentObsolete = 14, Miscellaneous = 15, Glyph = 16,
    Battlepet = 17, WoWToken = 18, Profession = 19,
  },
  -- item/TypeInfo.cpp
  ItemQuality = {
    Poor = 0, Common = 1, Uncommon = 2, Rare = 3, Epic = 4, Legendary = 5, Artifact = 6,
  },
  -- sound/ItemSound.cpp
  ItemSoundType = { Pickup = 0, Drop = 1, Use = 2, Close = 3 },
  -- model/DressUp.cpp
  ItemTryOnReason = { Success = 0, WrongRace = 1, NotEquippable = 2, DataPending = 3 },
  -- curve/Curve.cpp
  LuaCurveType = { Linear = 0, Step = 1, Cosine = 2, Cubic = 3 },
  -- model/SceneActor.cpp
  ModelBlendOperation = { None = 0, Anim = 1 },
  -- model/ModelInfo.cpp
  ModelSceneSetting = { AlignLightToOrbitDelta = 1 },
  -- model/ModelInfo.cpp
  ModelSceneType = {
    MountJournal = 0, PetJournalCard = 1, ShopCard = 2, EncounterJournal = 3,
    PetJournalLoadout = 4, ArtifactTier2 = 5, ArtifactTier2ForgingScene = 6,
    ArtifactTier2SlamEffect = 7, CommentatorVictoryFanfare = 8, ArtifactRelicTalentEffect = 9,
    PvPWarModeOrb = 10, PvPWarModeFire = 11, PartyPose = 12, AzeriteItemLevelUpToast = 13,
    AzeritePowers = 14, AzeriteRewardGlow = 15,
  },
  -- text/NumericFormatter.cpp
  NumericRuleFormatRounding = { Nearest = 0, Up = 1, Down = 2 },
  -- combat/Swing.cpp
  PlayerSwingType = { MainHand = 0, OffHand = 1, Ranged = 2 },
  -- unit/Power.cpp
  PowerType = {
    HealthCost = -2, None = -1, Mana = 0, Rage = 1, Focus = 2, Energy = 3, ComboPoints = 4,
    Runes = 5, RunicPower = 6, SoulShards = 7, LunarPower = 8, HolyPower = 9, Alternate = 10,
    Maelstrom = 11, Chi = 12, Insanity = 13, BurningEmbers = 14, DemonicFury = 15,
    ArcaneCharges = 16, Fury = 17, Pain = 18, Essence = 19, RuneBlood = 20, RuneFrost = 21,
    RuneUnholy = 22, AlternateQuest = 23, AlternateEncounter = 24, AlternateMount = 25,
    Balance = 26, Happiness = 27, ShadowOrbs = 28, RuneChromatic = 29,
  },
  -- quest/Info.cpp
  QuestClassification = {
    Important = 0, Legendary = 1, Campaign = 2, Calling = 3, Meta = 4, Recurring = 5,
    Questline = 6, Normal = 7, BonusObjective = 8, Threat = 9, WorldQuest = 10,
  },
  -- quest/Info.cpp
  QuestFrequency = { Default = 0, Daily = 1, Weekly = 2, ResetByScheduler = 3 },
  -- spell/ActivationOverlay.cpp
  ScreenLocationType = {
    Center = 0, Left = 1, Right = 2, Top = 3, Bottom = 4, TopLeft = 5, TopRight = 6,
    LeftOutside = 7, RightOutside = 8, LeftRight = 9, TopBottom = 10, LeftRightOutside = 11,
  },
  -- text/SecondsFormatter.cpp
  SecondsFormatterAbbreviation = { None = 0, Truncate = 1, OneLetter = 2 },
  -- text/SecondsFormatter.cpp
  SecondsFormatterInterval = { Seconds = 0, Minutes = 1, Hours = 2, Days = 3 },
  -- text/SecondsFormatter.cpp
  SecondsFormatterIntervalWhitespace = { Preserve = 0, Strip = 1, StripIgnoreLocale = 2 },
  -- death/SelfResurrect.cpp
  SelfResurrectOptionType = { Spell = 0, Item = 1 },
  -- spell/Info.cpp
  SpellBookItemType = { None = 0, Spell = 1, FutureSpell = 2, PetAction = 3, Flyout = 4 },
  -- spell/Info.cpp
  SpellBookSpellBank = { Player = 0, Pet = 1 },
  -- statusbar/Fill.cpp
  StatusBarFillStyle = { Standard = 0, StandardNoRangeFill = 1, Center = 2, Reverse = 3 },
  -- statusbar/Display.cpp
  StatusBarInterpolation = { Immediate = 0, ExponentialEaseOut = 1 },
  -- statusbar/Display.cpp
  StatusBarTimerDirection = { ElapsedTime = 0, RemainingTime = 1 },
  -- model/SceneActor.cpp
  TransmogOutfitSlotOptionSheatheCategory = { Default = 0, Back = 1, Side = 2, Hide = 3 },
  -- cursor/Changed.cpp
  UICursorType = {
    Default = 0, Item = 1, Money = 2, Spell = 3, PetAction = 4, Merchant = 5, ActionBar = 6,
    Macro = 7, Ammo = 8, Pet = 9, GuildBank = 10, GuildBankMoney = 11, EquipmentSet = 12,
    Currency = 13, Flyout = 14, VoidItem = 15, BattlePet = 16, Mount = 17, Toy = 18,
    ConduitCollectionItem = 19, PerksProgramVendorItem = 20,
  },
  -- model/ModelInfo.cpp
  UIModelSceneActorFlag = {
    Deprecated1 = 1, UseCenterForOriginX = 2, UseCenterForOriginY = 4, UseCenterForOriginZ = 8,
  },
  -- model/ModelInfo.cpp
  UIModelSceneContext = { None = -1, PerksProgram = 0 },
  -- model/ModelInfo.cpp
  UIModelSceneFlags = { SheatheWeapon = 1, HideWeapon = 2, Autodress = 4, NoCameraSpin = 8 },
  -- texture/Slice.cpp
  UITextureSliceMode = { Stretched = 0, Tiled = 1 },
  -- aura/Api.cpp
  UnitAuraSortDirection = { Normal = 0, Reverse = 1 },
  -- aura/Api.cpp
  UnitAuraSortRule = {
    Unsorted = 0, Default = 1, BigDefensive = 2, Expiration = 3, ExpirationOnly = 4, Name = 5,
    NameOnly = 6,
  },
  -- sound/VocalError.cpp
  Vocalerrorsounds = {
    Inventoryfull = 0, Outofammo = 1, NoequipLevel = 2, NoequipEver = 3, BoundNodrop = 4,
    Itemcooling = 5, Cantdrinkmore = 6, Canteatmore = 7, Cantinvite = 8, Inviteebusy = 9,
    Targettoofar = 10, Invalidtarget = 11, Spellcooling = 12, CantlearnLevel = 13, Locked = 14,
    Nomana = 15, Notwhiledead = 16, Cantloot = 17, Cantcreate = 18, Declinegroup = 19,
    Alreadyingroup = 20, Alreadyinguild = 21, Cantaffordbankslot = 22, Toomanybankslots = 23,
    CanteatMoving = 24, Notabag = 25, Cantputbag = 26, Wrongslot = 27, Ammoonlyinbag = 28,
    Bagfull = 29, Itemmaxcount = 30, CantlootDidntkill = 31, CantlootWrongfacing = 32,
    CantlootLocked = 33, CantlootNotstandingObsolete = 34, CantlootToofar = 35,
    Cantattackrongdirection = 36, CantattackNotstandingObsolete = 37, CantattackNotarget = 38,
    Notenoughgold = 39, Notenoughmoney = 40, Cantequip2HSkill = 41, Cantequip2Hequipped = 42,
    Cantequip2HNoskill = 43, Notequippable = 44, Genericnotarget = 45, CantcastOutofrange = 46,
    Potioncooling = 47, Proficiencyneeded = 48, Mustequippitem = 49, Abilitycooling = 50,
    Cantuseitem = 51, Chestinuse = 52, FoodcoolingObsolete = 53, CanttaxiNomoney = 54,
    Cantuselocked = 55, Noequipslotavailable = 56, Cantusetoofar = 57, Cantswap = 58,
    CanttradeSoulbound = 59, Cantflyhere = 60, Itemlocked = 61, Guildpermissions = 62, Norage = 63,
    Noenergy = 64, Noessence = 65, Invaliditemtarget = 66, ExhaustedObsolete = 67,
  },
}
for name, values in pairs(enums) do Enum[name] = values end
