# Where this layer differs from the ClassicAPI and Nampower DLLs

Every written deviation of the layer, one per `Deviation:` comment at its code; the comment is
the source and this list is generated from it (`benilla-pfui-plan/tools/deviations.py`).
`compatibility_notes_name_every_deviation` fails when a file with a deviation is missing here.
Deliberately unbuilt features and open hand checks live in the plan's `OPEN.md`.

- **app / area_loot** (`crates/benilla-app/src/pfui/area_loot.rs`): ClassicAPI also forgets a skipped corpse when the quest log changes; here only leaving view does. A walk asked while a loot window is open ends at once with no corpse.
- **app / soft_target** (`crates/benilla-app/src/pfui/soft_target/mod.rs`): 1.12 has no loose targets, so every target counts as one the player chose (locked); the model box is the bounding radius and the box height (`OverheadFallback`) times the scale, 1 x 2 yd for a GameObject, where ClassicAPI reads the model's bounds; the boss, critter, party and PvP classes are not built; the icons, circles, names, tooltip and nameplate dim, and the soft cast and soft attack, are not drawn or built.
- **app / priority** (`crates/benilla-app/src/pfui/soft_target/priority.rs`): party and raid membership are not classified yet; nothing sets these.
- **pfui / death_recap** (`crates/benilla-pfui/src/death_recap.rs`): a drain's amount is read as ClassicAPI reads it (its power type), and its overkill is 0, as this log carries no alternate power type.
- **pfui / duration_mods** (`crates/benilla-pfui/src/duration_mods.rs`): ClassicAPI also arms a trigger's targets so a rule re-applies to an aura the trigger itself creates moments later (Bestial Wrath's Scent of Blood); here a rule edits only an aura that is already there.
- **pfui / globals** (`crates/benilla-pfui/src/globals.lua`): ClassicAPI also takes ItemSubClass.dbc's short name ("Sword" for "One-Handed Swords"); the item info here carries only the long one.
- **pfui / modern_widgets** (`crates/benilla-pfui/src/modern_widgets.lua`): kept, not applied: this engine's drag always rounds to the value step, the `true` behaviour, so a slider set to false still steps.
- **pfui / states** (`crates/benilla-pfui/src/spell/states.rs`): scan actual stealth auras instead of the reference DLL's ambiguous visibility bit, which also describes mounts and some druid forms.
- **ui / area_loot** (`crates/benilla-ui/src/script/pfui_host/area_loot.rs`): that right-click also selects the corpse, where ClassicAPI's sender (`0x5df130`) leaves the target alone.
- **ui / encounter** (`crates/benilla-ui/src/script/pfui_host/encounter.rs`): ClassicAPI also writes ENCOUNTER_START / ENCOUNTER_END lines to the combat log file, which this client does not write.
- **ui / gossip_info** (`crates/benilla-ui/src/script/pfui_host/gossip_info.rs`): ClassicAPI also serves a quest-greeting window (`SMSG_QUESTGIVER_QUEST_LIST`) through the quest lists; here those lists stay empty for it. `questInfoID` is 0, ClassicAPI's answer for a quest not in the cache, and the gossip points of interest are never set.
- **ui / line_of_sight** (`crates/benilla-ui/src/script/pfui_host/line_of_sight.rs`): the verdict is at most 100 ms old (the host's pass), where ClassicAPI traces on the call.
- **ui / model_display** (`crates/benilla-ui/src/script/pfui_host/model_display.rs`): ClassicAPI's `SetCreature(creatureID [, displayID])` is not built: it reads the creature cache's record, which the VM does not hold.
- **ui / mouse_button** (`crates/benilla-ui/src/script/pfui_host/mouse_button.rs`): ClassicAPI answers "LeftButton" for a macro a `SetBindingMacro` key runs; this client does not tell the macro run its start, so that macro reads nil too.
- **ui / override_bindings** (`crates/benilla-ui/src/script/pfui_host/override_bindings.rs`): ClassicAPI's typed forms (SetOverrideBindingSpell/Item/Macro/Click) and the typed commands they bind are not built; nor is 4.3.4's reduced-key walk in the dispatch, which keeps 1.12's exact chord and one retry.
