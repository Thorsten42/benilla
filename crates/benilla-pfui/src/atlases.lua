-- ClassicAPI's built-in atlases (texture/AtlasData.h, v1.23.9): each Blizzard atlas name bound to
-- the texture that carries the same art and the sub-rect within it, mostly 1.12 files and the
-- textures ClassicAPI ships (served from its addon's Textures folder, LooseFiles::classicapi_builtin).
-- Rows: name, file, width, height, left, right, top, bottom, atlas id, element id, tiles
-- horizontally, tiles vertically, transposed, slice {left, top, right, bottom, mode}.
local define = ...
define("chatframe-button-highlightalert", "Interface\\ChatFrame\\UI-ChatIcon-BlinkHilight",
  32, 32, 0, 1, 0, 1, 924, 10841, false, false, false, nil)
define("MinimapArrow", "Interface\\Minimap\\MinimapArrow",
  32, 32, 0, 1, 0, 1, 647, 9441, false, false, false, nil)
define("Repair", "Interface\\CURSOR\\Repair",
  32, 32, 0, 1, 0, 1, 647, 9509, false, false, false, nil)
define("Rotating-MinimapArrow", "Interface\\Minimap\\ROTATING-MINIMAPARROW",
  32, 32, 0, 1, 0, 1, 647, 9442, false, false, false, nil)
define("Rotating-MinimapGroupArrow", "Interface\\Minimap\\Rotating-MinimapGroupArrow",
  32, 32, 0, 1, 0, 1, 647, 9443, false, false, false, nil)
define("Rotating-MinimapGuideArrow", "Interface\\Minimap\\ROTATING-MINIMAPGUIDEARROW",
  32, 32, 0, 1, 0, 1, 647, 9444, false, false, false, nil)
define("UI-ClassicScrollBar-EndCap-Top", "Interface\\PaperDollInfoFrame\\UI-Character-ScrollBar",
  31, 28, 0, 0.484375, 0, 0.109375, 1941, 18426, false, false, false, nil)
define("UI-ClassicScrollBar-EndCap-Bottom", "Interface\\PaperDollInfoFrame\\UI-Character-ScrollBar",
  31, 28, 0.515625, 1, 0.88671875, 0.99609375, 1941, 18425, false, false, false, nil)
define("UI-ClassicScrollBar-Center", "Interface\\PaperDollInfoFrame\\UI-Character-ScrollBar",
  31, 228, 0, 0.484375, 0.109375, 1, 1763, 18422, false, true, false, nil)
define("UI-ScrollBar-ClassicScrollUpButton-Up", "Interface\\Buttons\\UI-ScrollBar-ScrollUpButton-Up",
  19, 17, 0.1875, 0.78125, 0.21875, 0.75, 1941, 18434, false, false, false, nil)
define("UI-ScrollBar-ClassicScrollUpButton-Down", "Interface\\Buttons\\UI-ScrollBar-ScrollUpButton-Down",
  19, 17, 0.1875, 0.78125, 0.21875, 0.75, 1941, 18432, false, false, false, nil)
define("UI-ScrollBar-ClassicScrollUpButton-Disabled", "Interface\\Buttons\\UI-ScrollBar-ScrollUpButton-Disabled",
  19, 17, 0.1875, 0.78125, 0.21875, 0.75, 1941, 18431, false, false, false, nil)
define("UI-ScrollBar-ClassicScrollUpButton-Highlight", "Interface\\Buttons\\UI-ScrollBar-ScrollUpButton-Highlight",
  19, 17, 0.1875, 0.78125, 0.21875, 0.75, 1941, 18433, false, false, false, nil)
define("UI-ScrollBar-ClassicScrollDownButton-Up", "Interface\\Buttons\\UI-ScrollBar-ScrollDownButton-Up",
  19, 17, 0.1875, 0.78125, 0.21875, 0.75, 1941, 18430, false, false, false, nil)
define("UI-ScrollBar-ClassicScrollDownButton-Down", "Interface\\Buttons\\UI-ScrollBar-ScrollDownButton-Down",
  19, 17, 0.1875, 0.78125, 0.21875, 0.75, 1941, 18428, false, false, false, nil)
define("UI-ScrollBar-ClassicScrollDownButton-Disabled", "Interface\\Buttons\\UI-ScrollBar-ScrollDownButton-Disabled",
  19, 17, 0.1875, 0.78125, 0.21875, 0.75, 1941, 18427, false, false, false, nil)
define("UI-ScrollBar-ClassicScrollDownButton-Highlight", "Interface\\Buttons\\UI-ScrollBar-ScrollDownButton-Highlight",
  19, 17, 0.1875, 0.78125, 0.21875, 0.75, 1941, 18429, false, false, false, nil)
define("UI-ScrollBar-EndCap-Left", "Interface\\PaperDollInfoFrame\\UI-Character-ScrollBar",
  28, 31, 0, 0.484375, 0, 0.109375, 1769, 17384, false, false, true, nil)
define("UI-ScrollBar-EndCap-Right", "Interface\\PaperDollInfoFrame\\UI-Character-ScrollBar",
  28, 31, 0.515625, 1, 0.88671875, 0.99609375, 1769, 17385, false, false, true, nil)
define("_UI-ScrollBar-Center", "Interface\\PaperDollInfoFrame\\UI-Character-ScrollBar",
  228, 31, 0, 0.484375, 0.109375, 1, 1770, 17418, true, false, true, nil)
define("UI-ScrollBar-ScrollLeftButton-Up", "Interface\\Buttons\\UI-ScrollBar-ScrollUpButton-Up",
  17, 19, 0.1875, 0.78125, 0.21875, 0.75, 1769, 17406, false, false, true, nil)
define("UI-ScrollBar-ScrollLeftButton-Down", "Interface\\Buttons\\UI-ScrollBar-ScrollUpButton-Down",
  17, 19, 0.1875, 0.78125, 0.21875, 0.75, 1769, 17404, false, false, true, nil)
define("UI-ScrollBar-ScrollLeftButton-Disabled", "Interface\\Buttons\\UI-ScrollBar-ScrollUpButton-Disabled",
  17, 19, 0.1875, 0.78125, 0.21875, 0.75, 1769, 17403, false, false, true, nil)
define("UI-ScrollBar-ScrollLeftButton-Highlight", "Interface\\Buttons\\UI-ScrollBar-ScrollUpButton-Highlight",
  17, 19, 0.1875, 0.78125, 0.21875, 0.75, 1769, 17405, false, false, true, nil)
define("UI-ScrollBar-ScrollRightButton-Up", "Interface\\Buttons\\UI-ScrollBar-ScrollDownButton-Up",
  17, 19, 0.1875, 0.78125, 0.21875, 0.75, 1769, 17410, false, false, true, nil)
define("UI-ScrollBar-ScrollRightButton-Down", "Interface\\Buttons\\UI-ScrollBar-ScrollDownButton-Down",
  17, 19, 0.1875, 0.78125, 0.21875, 0.75, 1769, 17408, false, false, true, nil)
define("UI-ScrollBar-ScrollRightButton-Disabled", "Interface\\Buttons\\UI-ScrollBar-ScrollDownButton-Disabled",
  17, 19, 0.1875, 0.78125, 0.21875, 0.75, 1769, 17407, false, false, true, nil)
define("UI-ScrollBar-ScrollRightButton-Highlight", "Interface\\Buttons\\UI-ScrollBar-ScrollDownButton-Highlight",
  17, 19, 0.1875, 0.78125, 0.21875, 0.75, 1769, 17409, false, false, true, nil)
define("common-button-square-gray-up", "Interface\\Common\\ModelSceneControl",
  42, 42, 0, 0.5, 0, 0.5, 2666, 25907, false, false, false, nil)
define("common-button-square-gray-down", "Interface\\Common\\ModelSceneControl",
  42, 42, 0.5, 1, 0, 0.5, 2666, 25906, false, false, false, nil)
define("common-icon-zoomin", "Interface\\Common\\ModelSceneControl",
  25, 25, 0, 0.25, 0.5, 0.75, 1563, 11970, false, false, false, nil)
define("common-icon-zoomout", "Interface\\Common\\ModelSceneControl",
  25, 25, 0.25, 0.5, 0.5, 0.75, 1563, 11971, false, false, false, nil)
define("common-icon-rotateleft", "Interface\\Common\\ModelSceneControl",
  20, 20, 0.5, 0.75, 0.5, 0.75, 1563, 11967, false, false, false, nil)
define("common-icon-rotateright", "Interface\\Common\\ModelSceneControl",
  20, 20, 0.75, 1, 0.5, 0.75, 1563, 11968, false, false, false, nil)
define("common-icon-undo", "Interface\\Common\\ModelSceneControl",
  25, 25, 0, 0.25, 0.75, 1, 1563, 11969, false, false, false, nil)
define("ui-damagemeters-header-bar", "Interface\\DamageMeter\\DamageMeter",
  280, 28, 0.322265625, 0.869140625, 0.58984375, 0.69921875, 3658, 32444,
  false, false, false, { 80, 14, 80, 22, 0 })
define("damagemeters-background", "Interface\\DamageMeter\\DamageMeter",
  154, 148, 0.001953125, 0.302734375, 0.00390625, 0.58203125, 3658, 32562,
  false, false, false, { 20, 80, 20, 20, 0 })
define("damagemeters-scalehandle", "Interface\\DamageMeter\\DamageMeter",
  60, 60, 0.443359375, 0.560546875, 0.00390625, 0.23828125, 3658, 32492, false, false, false, nil)
define("damagemeters-scalehandle-hover", "Interface\\DamageMeter\\DamageMeter",
  60, 60, 0.564453125, 0.681640625, 0.00390625, 0.23828125, 3658, 32563, false, false, false, nil)
define("damagemeters-scalehandle-pressed", "Interface\\DamageMeter\\DamageMeter",
  60, 60, 0.685546875, 0.802734375, 0.00390625, 0.23828125, 3658, 32564, false, false, false, nil)
define("ui-hud-cooldownmanager-bar", "Interface\\DamageMeter\\DamageMeter",
  124, 10, 0.255859375, 0.498046875, 0.86328125, 0.90234375, 3147, 30193,
  false, false, false, { 4, 5, 4, 4, 0 })
define("ui-hud-cooldownmanager-bar-bg", "Interface\\DamageMeter\\DamageMeter",
  132, 19, 0.228515625, 0.486328125, 0.75, 0.82421875, 3147, 30191,
  false, false, false, { 121, 7, 10, 11, 1 })
define("ui-damagemeters-bar-shadowbg", "Interface\\DamageMeter\\DamageMeter",
  128, 14, 0.685546875, 0.935546875, 0.75, 0.8046875, 3658, 32496,
  false, false, false, { 24, 10, 24, 10, 0 })
define("ui-damagemeters-bar-shadowedge", "Interface\\DamageMeter\\DamageMeter",
  128, 14, 0.001953125, 0.251953125, 0.86328125, 0.91796875, 3658, 32443,
  false, false, false, { 4, 5, 4, 4, 0 })
define("ui-questtrackerbutton-collapse-all", "Interface\\DamageMeter\\DamageMeter",
  18, 19, 0.490234375, 0.525390625, 0.75, 0.82421875, 2547, 12246, false, false, false, nil)
define("ui-questtrackerbutton-collapse-all-pressed", "Interface\\DamageMeter\\DamageMeter",
  18, 19, 0.529296875, 0.564453125, 0.75, 0.82421875, 2547, 12245, false, false, false, nil)
define("ui-questtrackerbutton-expand-all", "Interface\\DamageMeter\\DamageMeter",
  18, 19, 0.568359375, 0.603515625, 0.75, 0.82421875, 2547, 12250, false, false, false, nil)
define("ui-questtrackerbutton-expand-all-pressed", "Interface\\DamageMeter\\DamageMeter",
  18, 19, 0.607421875, 0.642578125, 0.75, 0.82421875, 2547, 12249, false, false, false, nil)
define("ui-questtrackerbutton-red-highlight", "Interface\\DamageMeter\\DamageMeter",
  18, 19, 0.646484375, 0.681640625, 0.75, 0.82421875, 2547, 24977, false, false, false, nil)
define("common-dropdown-bg", "Interface\\DamageMeter\\DamageMeter",
  68, 68, 0.306640625, 0.439453125, 0.00390625, 0.26953125, 2634, 25557,
  false, false, false, { 16, 13, 16, 19, 0 })
define("common-dropdown-a-button-settings-disabled-shadowless", "Interface\\DamageMeter\\DamageMeter",
  27, 27, 0.873046875, 0.92578125, 0.58984375, 0.6953125, 3678, 32612, false, false, false, nil)
define("common-dropdown-a-button-settings-shadowless", "Interface\\DamageMeter\\DamageMeter",
  27, 27, 0.9296875, 0.982421875, 0.58984375, 0.6953125, 3678, 32617, false, false, false, nil)
define("common-dropdown-a-button-settings-pressed-shadowless", "Interface\\DamageMeter\\DamageMeter",
  27, 27, 0.001953125, 0.0546875, 0.75, 0.85546875, 3678, 32615, false, false, false, nil)
define("common-dropdown-a-button-settings-hover-shadowless", "Interface\\DamageMeter\\DamageMeter",
  27, 27, 0.05859375, 0.111328125, 0.75, 0.85546875, 3678, 32613, false, false, false, nil)
define("common-dropdown-a-button-settings-pressedhover-shadowless", "Interface\\DamageMeter\\DamageMeter",
  27, 27, 0.115234375, 0.16796875, 0.75, 0.85546875, 3678, 32616, false, false, false, nil)
define("common-dropdown-a-button-settings-open-shadowless", "Interface\\DamageMeter\\DamageMeter",
  27, 27, 0.171875, 0.224609375, 0.75, 0.85546875, 3678, 32614, false, false, false, nil)
define("common-dropdown-c-button", "Interface\\DamageMeter\\DamageMeter",
  39, 39, 0.806640625, 0.8828125, 0.00390625, 0.15625, 2634, 25502,
  false, false, false, { 12, 0, 12, 0, 0 })
define("common-dropdown-c-button-hover-1", "Interface\\DamageMeter\\DamageMeter",
  39, 39, 0.88671875, 0.962890625, 0.00390625, 0.15625, 2634, 25728,
  false, false, false, { 12, 0, 12, 0, 0 })
define("common-dropdown-c-button-pressed-1", "Interface\\DamageMeter\\DamageMeter",
  39, 39, 0.001953125, 0.078125, 0.58984375, 0.7421875, 2634, 25730,
  false, false, false, { 14, 0, 12, 0, 0 })
define("common-dropdown-c-button-pressedhover-1", "Interface\\DamageMeter\\DamageMeter",
  39, 39, 0.08203125, 0.158203125, 0.58984375, 0.7421875, 2634, 25732,
  false, false, false, { 14, 0, 12, 0, 0 })
define("common-dropdown-c-button-open", "Interface\\DamageMeter\\DamageMeter",
  39, 39, 0.162109375, 0.23828125, 0.58984375, 0.7421875, 2634, 25513,
  false, false, false, { 12, 0, 12, 0, 0 })
define("common-dropdown-c-button-disabled", "Interface\\DamageMeter\\DamageMeter",
  39, 39, 0.2421875, 0.318359375, 0.58984375, 0.7421875, 2634, 25591,
  false, false, false, { 12, 0, 12, 0, 0 })
define("common-dropdown-c-button-hover-arrow", "Interface\\DamageMeter\\DamageMeter",
  12, 5, 0.501953125, 0.525390625, 0.86328125, 0.8828125, 2634, 25565, false, false, false, nil)
define("classicon-warrior", "Interface\\DamageMeter\\DamageMeterIcons",
  128, 128, 0.00390625, 0.25390625, 0.00390625, 0.25390625, 897, 12069, false, false, false, nil)
define("classicon-paladin", "Interface\\DamageMeter\\DamageMeterIcons",
  128, 128, 0.26171875, 0.51171875, 0.00390625, 0.25390625, 897, 12064, false, false, false, nil)
define("classicon-hunter", "Interface\\DamageMeter\\DamageMeterIcons",
  128, 128, 0.51953125, 0.76953125, 0.00390625, 0.25390625, 897, 12061, false, false, false, nil)
define("classicon-rogue", "Interface\\DamageMeter\\DamageMeterIcons",
  128, 128, 0.00390625, 0.25390625, 0.26171875, 0.51171875, 897, 12066, false, false, false, nil)
define("classicon-priest", "Interface\\DamageMeter\\DamageMeterIcons",
  128, 128, 0.26171875, 0.51171875, 0.26171875, 0.51171875, 897, 12065, false, false, false, nil)
define("classicon-shaman", "Interface\\DamageMeter\\DamageMeterIcons",
  128, 128, 0.51953125, 0.76953125, 0.26171875, 0.51171875, 897, 12067, false, false, false, nil)
define("classicon-mage", "Interface\\DamageMeter\\DamageMeterIcons",
  128, 128, 0.00390625, 0.25390625, 0.51953125, 0.76953125, 897, 12062, false, false, false, nil)
define("classicon-warlock", "Interface\\DamageMeter\\DamageMeterIcons",
  128, 128, 0.26171875, 0.51171875, 0.51953125, 0.76953125, 897, 12068, false, false, false, nil)
define("classicon-druid", "Interface\\DamageMeter\\DamageMeterIcons",
  128, 128, 0.51953125, 0.76953125, 0.51953125, 0.76953125, 897, 12060, false, false, false, nil)
define("nameplates-icon-elite-gold", "Interface\\DamageMeter\\DamageMeterIcons",
  40, 40, 0.77734375, 0.90234375, 0.51953125, 0.64453125, 731, 5444, false, false, false, nil)
define("nameplates-icon-elite-silver", "Interface\\DamageMeter\\DamageMeterIcons",
  40, 40, 0.00390625, 0.12890625, 0.77734375, 0.90234375, 731, 5470, false, false, false, nil)
define("ui-hud-unitframe-target-portraiton-boss-rare-star", "Interface\\DamageMeter\\DamageMeterIcons",
  20, 20, 0.40234375, 0.50390625, 0.77734375, 0.87890625, 2130, 20547, false, false, false, nil)
define("hordesymbol", "Interface\\DamageMeter\\DamageMeterIcons",
  32, 32, 0.13671875, 0.26171875, 0.77734375, 0.90234375, 647, 6902, false, false, false, nil)
define("alliancesymbol", "Interface\\DamageMeter\\DamageMeterIcons",
  32, 32, 0.26953125, 0.39453125, 0.77734375, 0.90234375, 647, 6901, false, false, false, nil)
define("UI-Frame-PortraitMetal-CornerTopLeft", "Interface\\FrameGeneral\\UIFrameMetal",
  75, 75, 0.00390625, 0.296875, 0.00390625, 0.296875, 1390, 7672, false, false, false, nil)
define("UI-Frame-Metal-CornerTopLeft", "Interface\\FrameGeneral\\UIFrameMetal",
  75, 75, 0.3046875, 0.59765625, 0.00390625, 0.296875, 1390, 7678, false, false, false, nil)
define("UI-Frame-Metal-CornerTopRight", "Interface\\FrameGeneral\\UIFrameMetal",
  75, 75, 0.60546875, 0.8984375, 0.00390625, 0.296875, 1390, 7679, false, false, false, nil)
define("UI-Frame-Metal-CornerTopRightDouble", "Interface\\FrameGeneral\\UIFrameMetal",
  75, 75, 0.00390625, 0.296875, 0.3046875, 0.59765625, 1390, 7715, false, false, false, nil)
define("UI-Frame-Metal-CornerBottomLeft", "Interface\\FrameGeneral\\UIFrameMetal",
  32, 32, 0.5625, 0.6875, 0.3046875, 0.4296875, 1390, 7674, false, false, false, nil)
define("UI-Frame-Metal-CornerBottomRight", "Interface\\FrameGeneral\\UIFrameMetal",
  32, 32, 0.6953125, 0.8203125, 0.3046875, 0.4296875, 1390, 7677, false, false, false, nil)
define("UI-Frame-SimpleMetal-CornerTopLeft", "Interface\\FrameGeneral\\UIFrameMetal",
  64, 64, 0.3046875, 0.5546875, 0.3046875, 0.5546875, 1380, 7682, false, false, false, nil)
define("UI-Frame-InnerTopLeft", "Interface\\FrameGeneral\\UIFrameMetal",
  6, 6, 0.96875, 0.9921875, 0.3046875, 0.328125, 948, 6379, false, false, false, nil)
define("UI-Frame-InnerTopRight", "Interface\\FrameGeneral\\UIFrameMetal",
  6, 6, 0.00390625, 0.02734375, 0.60546875, 0.62890625, 948, 6389, false, false, false, nil)
define("UI-Frame-InnerBotLeftCorner", "Interface\\FrameGeneral\\UIFrameMetal",
  6, 6, 0.03515625, 0.05859375, 0.60546875, 0.62890625, 948, 6381, false, false, false, nil)
define("UI-Frame-InnerBotRight", "Interface\\FrameGeneral\\UIFrameMetal",
  6, 6, 0.06640625, 0.08984375, 0.60546875, 0.62890625, 948, 6388, false, false, false, nil)
define("uiframebackground-nineslice-cornerbottomleft", "Interface\\FrameGeneral\\UIFrameMetal",
  16, 16, 0.828125, 0.890625, 0.3046875, 0.3671875, 2120, 20550, false, false, false, nil)
define("uiframebackground-nineslice-cornerbottomright", "Interface\\FrameGeneral\\UIFrameMetal",
  16, 16, 0.8984375, 0.9609375, 0.3046875, 0.3671875, 2120, 20551, false, false, false, nil)
define("_UI-Frame-Metal-EdgeTop", "Interface\\FrameGeneral\\UIFrameMetalEdgeTop",
  32, 75, 0, 1, 0.0078125, 0.59375, 1395, 7676, true, false, false, nil)
define("_UI-Frame-Metal-EdgeBottom", "Interface\\FrameGeneral\\UIFrameMetalEdgeBottom",
  16, 32, 0, 1, 0.015625, 0.515625, 1395, 7673, true, false, false, nil)
define("!UI-Frame-Metal-EdgeLeft", "Interface\\FrameGeneral\\UIFrameMetalEdgeSides",
  75, 16, 0.00390625, 0.296875, 0, 1, 1394, 7680, false, true, false, nil)
define("!UI-Frame-Metal-EdgeRight", "Interface\\FrameGeneral\\UIFrameMetalEdgeSides",
  75, 16, 0.3046875, 0.59765625, 0, 1, 1394, 7681, false, true, false, nil)
define("_UI-Frame-SimpleMetal-EdgeTop", "Interface\\FrameGeneral\\UIFrameSimpleMetalEdgeTop",
  64, 64, 0, 1, 0.0078125, 0.5078125, 1380, 7683, true, false, false, nil)
define("!UI-Frame-SimpleMetal-EdgeLeft", "Interface\\FrameGeneral\\UIFrameSimpleMetalEdgeLeft",
  64, 64, 0.0078125, 0.5078125, 0, 1, 1381, 7684, false, true, false, nil)
define("_UI-Frame-InnerTopTile", "Interface\\FrameGeneral\\UIFrameInsetEdgeTopBottom",
  256, 3, 0, 1, 0.0625, 0.25, 950, 6399, true, false, false, nil)
define("_UI-Frame-InnerBotTile", "Interface\\FrameGeneral\\UIFrameInsetEdgeTopBottom",
  256, 3, 0, 1, 0.375, 0.5625, 950, 6402, true, false, false, nil)
define("!UI-Frame-InnerLeftTile", "Interface\\FrameGeneral\\UIFrameInsetEdgeSides",
  3, 256, 0.0625, 0.25, 0, 1, 949, 6378, false, true, false, nil)
define("!UI-Frame-InnerRightTile", "Interface\\FrameGeneral\\UIFrameInsetEdgeSides",
  3, 256, 0.375, 0.5625, 0, 1, 949, 6396, false, true, false, nil)
define("common-dropdown-a-button", "Interface\\Common\\CommonDropdown",
  27, 27, 0.53125, 0.583984375, 0.00390625, 0.109375, 2634, 25556, false, false, false, nil)
define("common-dropdown-a-button-disabled", "Interface\\Common\\CommonDropdown",
  27, 27, 0.587890625, 0.640625, 0.00390625, 0.109375, 2634, 25588, false, false, false, nil)
define("common-dropdown-a-button-disabled-shadowless", "Interface\\Common\\CommonDropdown",
  27, 27, 0.64453125, 0.697265625, 0.00390625, 0.109375, 2634, 29007, false, false, false, nil)
define("common-dropdown-a-button-hover", "Interface\\Common\\CommonDropdown",
  27, 27, 0.701171875, 0.75390625, 0.00390625, 0.109375, 2634, 25553, false, false, false, nil)
define("common-dropdown-a-button-hover-shadowless", "Interface\\Common\\CommonDropdown",
  27, 27, 0.7578125, 0.810546875, 0.00390625, 0.109375, 2634, 29008, false, false, false, nil)
define("common-dropdown-a-button-open", "Interface\\Common\\CommonDropdown",
  27, 27, 0.814453125, 0.8671875, 0.00390625, 0.109375, 2634, 25559, false, false, false, nil)
define("common-dropdown-a-button-open-shadowless", "Interface\\Common\\CommonDropdown",
  27, 27, 0.87109375, 0.923828125, 0.00390625, 0.109375, 2634, 29009, false, false, false, nil)
define("common-dropdown-a-button-pressed", "Interface\\Common\\CommonDropdown",
  27, 27, 0.927734375, 0.98046875, 0.00390625, 0.109375, 2634, 25554, false, false, false, nil)
define("common-dropdown-a-button-pressed-shadowless", "Interface\\Common\\CommonDropdown",
  27, 27, 0.001953125, 0.0546875, 0.36328125, 0.46875, 2634, 29010, false, false, false, nil)
define("common-dropdown-a-button-pressedhover", "Interface\\Common\\CommonDropdown",
  27, 27, 0.05859375, 0.111328125, 0.36328125, 0.46875, 2634, 25555, false, false, false, nil)
define("common-dropdown-a-button-pressedhover-shadowless", "Interface\\Common\\CommonDropdown",
  27, 27, 0.115234375, 0.16796875, 0.36328125, 0.46875, 2634, 29011, false, false, false, nil)
define("common-dropdown-a-button-shadowless", "Interface\\Common\\CommonDropdown",
  27, 27, 0.171875, 0.224609375, 0.36328125, 0.46875, 2634, 29012, false, false, false, nil)
define("common-dropdown-b-button", "Interface\\Common\\CommonDropdown",
  97, 26, 0.228515625, 0.41796875, 0.36328125, 0.46484375, 2634, 25551,
  false, false, false, { 8, 0, 18, 0, 0 })
define("common-dropdown-b-button-disabled", "Interface\\Common\\CommonDropdown",
  97, 26, 0.421875, 0.611328125, 0.36328125, 0.46484375, 2634, 25589,
  false, false, false, { 8, 0, 18, 0, 0 })
define("common-dropdown-b-button-hover", "Interface\\Common\\CommonDropdown",
  97, 26, 0.615234375, 0.8046875, 0.36328125, 0.46484375, 2634, 25548,
  false, false, false, { 8, 0, 18, 0, 0 })
define("common-dropdown-b-button-open", "Interface\\Common\\CommonDropdown",
  97, 26, 0.80859375, 0.998046875, 0.36328125, 0.46484375, 2634, 25560,
  false, false, false, { 8, 0, 18, 0, 0 })
define("common-dropdown-b-button-pressed", "Interface\\Common\\CommonDropdown",
  97, 26, 0.001953125, 0.19140625, 0.4765625, 0.578125, 2634, 25549,
  false, false, false, { 10, 1, 17, 1, 0 })
define("common-dropdown-b-button-pressedhover", "Interface\\Common\\CommonDropdown",
  97, 26, 0.1953125, 0.384765625, 0.4765625, 0.578125, 2634, 25550,
  false, false, false, { 10, 0, 17, 0, 0 })
define("common-dropdown-c-bg", "Interface\\Common\\CommonDropdown",
  90, 90, 0.001953125, 0.177734375, 0.00390625, 0.35546875, 2634, 25590,
  false, false, false, { 23, 18, 23, 28, 0 })
define("common-dropdown-c-button-hover-2", "Interface\\Common\\CommonDropdown",
  39, 39, 0.291015625, 0.3671875, 0.00390625, 0.15625, 2634, 25729,
  false, false, false, { 12, 0, 12, 0, 0 })
define("common-dropdown-c-button-pressed-2", "Interface\\Common\\CommonDropdown",
  39, 39, 0.37109375, 0.447265625, 0.00390625, 0.15625, 2634, 25731,
  false, false, false, { 14, 0, 12, 0, 0 })
define("common-dropdown-c-button-pressedhover-2", "Interface\\Common\\CommonDropdown",
  39, 39, 0.451171875, 0.52734375, 0.00390625, 0.15625, 2634, 25733,
  false, false, false, { 14, 0, 12, 0, 0 })
define("common-dropdown-icon-checkmark-yellow", "Interface\\Common\\CommonDropdown",
  15, 14, 0.708984375, 0.73828125, 0.4765625, 0.53125, 2634, 25561, false, false, false, nil)
define("common-dropdown-icon-radialtick-yellow", "Interface\\Common\\CommonDropdown",
  18, 18, 0.482421875, 0.517578125, 0.4765625, 0.546875, 2634, 25562, false, false, false, nil)
define("common-dropdown-textholder", "Interface\\Common\\CommonDropdown",
  54, 41, 0.181640625, 0.287109375, 0.00390625, 0.1640625, 2634, 25558,
  false, false, false, { 16, 0, 19, 0, 0 })
define("common-dropdown-tickradial", "Interface\\Common\\CommonDropdown",
  18, 18, 0.521484375, 0.556640625, 0.4765625, 0.546875, 2634, 25563, false, false, false, nil)
define("common-dropdown-ticksquare", "Interface\\Common\\CommonDropdown",
  12, 12, 0.7421875, 0.765625, 0.4765625, 0.5234375, 2634, 25564, false, false, false, nil)
define("common-search-magnifyingglass", "Interface\\Common\\CommonDropdown",
  24, 24, 0.388671875, 0.435546875, 0.4765625, 0.5703125, 3172, 11711, false, false, false, nil)
define("common-dropdown-customize-mouseover", "Interface\\Common\\CommonDropdown",
  20, 20, 0.439453125, 0.478515625, 0.4765625, 0.5546875, 2634, 25927,
  false, false, false, { 4, 4, 4, 4, 0 })
define("common-dropdown-icon-next", "Interface\\Common\\CommonDropdown",
  17, 17, 0.560546875, 0.59375, 0.4765625, 0.54296875, 2634, 25737, false, false, false, nil)
define("common-dropdown-icon-next-disabled", "Interface\\Common\\CommonDropdown",
  17, 17, 0.59765625, 0.630859375, 0.4765625, 0.54296875, 2634, 25736, false, false, false, nil)
define("common-dropdown-icon-back", "Interface\\Common\\CommonDropdown",
  17, 17, 0.634765625, 0.66796875, 0.4765625, 0.54296875, 2634, 25735, false, false, false, nil)
define("common-dropdown-icon-back-disabled", "Interface\\Common\\CommonDropdown",
  17, 17, 0.671875, 0.705078125, 0.4765625, 0.54296875, 2634, 25734, false, false, false, nil)
define("Options_InnerFrame", "Interface\\OptionsFrame\\Options",
  886, 618, 0.0009765625, 0.8662109375, 0.146484375, 0.75, 733, 18651, false, false, false, nil)
define("Options_HorizontalDivider", "Interface\\OptionsFrame\\Options",
  630, 1, 0.0009765625, 0.6162109375, 0.1435546875, 0.14453125, 733, 18647,
  false, false, false, nil)
define("Options_List_Active", "Interface\\OptionsFrame\\Options",
  187, 21, 0.58984375, 0.7724609375, 0.0009765625, 0.021484375, 733, 18648,
  false, false, false, nil)
define("Options_List_Hover", "Interface\\OptionsFrame\\Options",
  187, 21, 0.7744140625, 0.95703125, 0.0009765625, 0.021484375, 733, 18649,
  false, false, false, nil)
define("Options_CategoryHeader_1", "Interface\\OptionsFrame\\Options",
  199, 144, 0.0009765625, 0.1953125, 0.0009765625, 0.1416015625, 733, 18644,
  false, false, false, nil)
define("Options_CategoryHeader_2", "Interface\\OptionsFrame\\Options",
  199, 144, 0.3935546875, 0.587890625, 0.0009765625, 0.1416015625, 733, 18645,
  false, false, false, nil)
define("Options_CategoryHeader_3", "Interface\\OptionsFrame\\Options",
  199, 144, 0.197265625, 0.3916015625, 0.0009765625, 0.1416015625, 733, 18646,
  false, false, false, nil)
define("Options_Tab_Left", "Interface\\OptionsFrame\\Options",
  7, 23, 0.5927734375, 0.599609375, 0.05078125, 0.0732421875, 733, 18671, false, false, false, nil)
define("Options_Tab_Middle", "Interface\\OptionsFrame\\Options",
  1, 23, 0.58984375, 0.5908203125, 0.0234375, 0.0458984375, 733, 18672, false, false, false, nil)
define("Options_Tab_Right", "Interface\\OptionsFrame\\Options",
  7, 23, 0.5927734375, 0.599609375, 0.1025390625, 0.125, 733, 18673, false, false, false, nil)
define("Options_Tab_Active_Left", "Interface\\OptionsFrame\\Options",
  7, 26, 0.5927734375, 0.599609375, 0.0234375, 0.048828125, 733, 18666, false, false, false, nil)
define("Options_Tab_Active_Middle", "Interface\\OptionsFrame\\Options",
  1, 26, 0.58984375, 0.5908203125, 0.0478515625, 0.0732421875, 733, 18667, false, false, false, nil)
define("Options_Tab_Active_Right", "Interface\\OptionsFrame\\Options",
  7, 26, 0.5927734375, 0.599609375, 0.0751953125, 0.1005859375, 733, 18668,
  false, false, false, nil)
define("checkbox-minimal", "Interface\\Common\\MinimalCheckbox",
  30, 29, 0.015625, 0.484375, 0.015625, 0.46875, 1983, 18779, false, false, false, nil)
define("checkmark-minimal", "Interface\\Common\\MinimalCheckbox",
  30, 29, 0.015625, 0.484375, 0.5, 0.953125, 1983, 18781, false, false, false, nil)
define("checkmark-minimal-disabled", "Interface\\Common\\MinimalCheckbox",
  30, 29, 0.515625, 0.984375, 0.015625, 0.46875, 1983, 18780, false, false, false, nil)
define("Options_ListExpand_Left", "Interface\\Options\\OptionsExpandListButton",
  12, 26, 0.0078125, 0.1015625, 0.65625, 0.859375, 1974, 18661, false, false, false, nil)
define("Options_ListExpand_Right", "Interface\\Options\\OptionsExpandListButton",
  28, 26, 0.0078125, 0.2265625, 0.4375, 0.640625, 1974, 18662, false, false, false, nil)
define("Options_ListExpand_Right_Expanded", "Interface\\Options\\OptionsExpandListButton",
  28, 26, 0.2421875, 0.4609375, 0.4375, 0.640625, 1974, 18659, false, false, false, nil)
define("_Options_ListExpand_Middle", "Interface\\Options\\OptionsExpandListButton",
  1, 26, 0, 0.0078125, 0.21875, 0.421875, 1974, 18660, true, false, false, nil)
define("common-button-dropdown-closed", "Interface\\Common\\CommonButtonsDropdown",
  22, 22, 0.015625, 0.359375, 0.015625, 0.359375, 1938, 18374, false, false, false, nil)
define("common-button-dropdown-closedpressed", "Interface\\Common\\CommonButtonsDropdown",
  22, 22, 0.390625, 0.734375, 0.015625, 0.359375, 1938, 18375, false, false, false, nil)
define("common-button-dropdown-open", "Interface\\Common\\CommonButtonsDropdown",
  22, 22, 0.015625, 0.359375, 0.390625, 0.734375, 1938, 18376, false, false, false, nil)
define("common-button-dropdown-openpressed", "Interface\\Common\\CommonButtonsDropdown",
  22, 22, 0.390625, 0.734375, 0.390625, 0.734375, 1938, 18377, false, false, false, nil)
define("Minimal_SliderBar_Left", "Interface\\Buttons\\MinimalSliderBar",
  11, 17, 0.4375, 0.78125, 0.3203125, 0.453125, 1973, 18655, false, false, false, nil)
define("Minimal_SliderBar_Right", "Interface\\Buttons\\MinimalSliderBar",
  11, 17, 0.03125, 0.375, 0.484375, 0.6171875, 1973, 18656, false, false, false, nil)
define("_Minimal_SliderBar_Middle", "Interface\\Buttons\\MinimalSliderBar",
  1, 17, 0, 0.03125, 0.0078125, 0.140625, 1973, 18657, true, false, false, nil)
define("Minimal_SliderBar_Button", "Interface\\Buttons\\MinimalSliderBar",
  20, 19, 0.03125, 0.65625, 0.15625, 0.3046875, 1973, 18652, false, false, false, nil)
define("Minimal_SliderBar_Button_Left", "Interface\\Buttons\\MinimalSliderBar",
  11, 19, 0.03125, 0.375, 0.3203125, 0.46875, 1973, 18653, false, false, false, nil)
define("Minimal_SliderBar_Button_Right", "Interface\\Buttons\\MinimalSliderBar",
  9, 18, 0.03125, 0.3125, 0.6328125, 0.7734375, 1973, 18654, false, false, false, nil)
define("RedButton-Exit", "Interface\\Buttons\\RedButtons",
  18, 19, 0.1640625, 0.3046875, 0.015625, 0.3125, 2196, 20574, false, false, false, nil)
define("RedButton-exit-pressed", "Interface\\Buttons\\RedButtons",
  18, 19, 0.1640625, 0.3046875, 0.671875, 0.96875, 2196, 20576, false, false, false, nil)
define("RedButton-Exit-Disabled", "Interface\\Buttons\\RedButtons",
  18, 19, 0.1640625, 0.3046875, 0.34375, 0.640625, 2196, 20575, false, false, false, nil)
define("RedButton-Highlight", "Interface\\Buttons\\RedButtons",
  18, 19, 0.4765625, 0.6171875, 0.015625, 0.3125, 2196, 20580, false, false, false, nil)
define("minimal-scrollbar-arrow-top", "Interface\\Buttons\\MinimalScrollbarProportional",
  17, 11, 0.6875, 0.8203125, 0.015625, 0.1875, 1921, 18212, false, false, false, nil)
define("minimal-scrollbar-arrow-top-over", "Interface\\Buttons\\MinimalScrollbarProportional",
  17, 11, 0.390625, 0.5234375, 0.21875, 0.390625, 1921, 18229, false, false, false, nil)
define("minimal-scrollbar-arrow-top-down", "Interface\\Buttons\\MinimalScrollbarProportional",
  17, 11, 0.8359375, 0.96875, 0.015625, 0.1875, 1921, 18228, false, false, false, nil)
define("minimal-scrollbar-arrow-bottom", "Interface\\Buttons\\MinimalScrollbarProportional",
  17, 11, 0.2421875, 0.375, 0.8125, 0.984375, 1921, 18211, false, false, false, nil)
define("minimal-scrollbar-arrow-bottom-over", "Interface\\Buttons\\MinimalScrollbarProportional",
  17, 11, 0.5390625, 0.671875, 0.015625, 0.1875, 1921, 18227, false, false, false, nil)
define("minimal-scrollbar-arrow-bottom-down", "Interface\\Buttons\\MinimalScrollbarProportional",
  17, 11, 0.390625, 0.5234375, 0.015625, 0.1875, 1921, 18226, false, false, false, nil)
define("minimal-scrollbar-track-top", "Interface\\Buttons\\MinimalScrollbarProportional",
  8, 8, 0.1640625, 0.2265625, 0.609375, 0.734375, 1921, 18216, false, false, false, nil)
define("minimal-scrollbar-track-bottom", "Interface\\Buttons\\MinimalScrollbarProportional",
  8, 8, 0.0859375, 0.1484375, 0.765625, 0.890625, 1921, 18215, false, false, false, nil)
define("!minimal-scrollbar-track-middle", "Interface\\Buttons\\MinimalScrollbarVertical",
  8, 1, 0.015625, 0.140625, 0, 0.0009765625, 1922, 18217, false, true, false, nil)
define("minimal-scrollbar-small-thumb-top", "Interface\\Buttons\\MinimalScrollbarSmallProportional",
  8, 8, 0.3125, 0.4375, 0.84375, 0.96875, 2404, 23501, false, false, false, nil)
define("minimal-scrollbar-small-thumb-bottom", "Interface\\Buttons\\MinimalScrollbarSmallProportional",
  8, 8, 0.609375, 0.734375, 0.484375, 0.609375, 2404, 23498, false, false, false, nil)
define("minimal-scrollbar-small-thumb-top-over", "Interface\\Buttons\\MinimalScrollbarSmallProportional",
  8, 8, 0.46875, 0.59375, 0.84375, 0.96875, 2404, 23500, false, false, false, nil)
define("minimal-scrollbar-small-thumb-bottom-over", "Interface\\Buttons\\MinimalScrollbarSmallProportional",
  8, 8, 0.3125, 0.4375, 0.6875, 0.8125, 2404, 23497, false, false, false, nil)
define("minimal-scrollbar-small-thumb-top-down", "Interface\\Buttons\\MinimalScrollbarSmallProportional",
  8, 8, 0.46875, 0.59375, 0.6875, 0.8125, 2404, 23499, false, false, false, nil)
define("minimal-scrollbar-small-thumb-bottom-down", "Interface\\Buttons\\MinimalScrollbarSmallProportional",
  8, 8, 0.765625, 0.890625, 0.484375, 0.609375, 2404, 23496, false, false, false, nil)
define("minimal-scrollbar-small-thumb-middle", "Interface\\Buttons\\MinimalScrollbarSmallVertical",
  8, 715, 0.484375, 0.609375, 0.0009765625, 0.69921875, 2403, 23486, false, false, false, nil)
define("minimal-scrollbar-small-thumb-middle-over", "Interface\\Buttons\\MinimalScrollbarSmallVertical",
  8, 715, 0.328125, 0.453125, 0.0009765625, 0.69921875, 2403, 23485, false, false, false, nil)
define("minimal-scrollbar-small-thumb-middle-down", "Interface\\Buttons\\MinimalScrollbarSmallVertical",
  8, 715, 0.171875, 0.296875, 0.0009765625, 0.69921875, 2403, 23484, false, false, false, nil)
define("common-search-clearbutton", "Interface\\Common\\CommonSearchClearButton",
  20, 20, 0.03125, 0.65625, 0.03125, 0.65625, 3172, 11710, false, false, false, nil)
