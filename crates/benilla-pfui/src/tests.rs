use benilla_ui::script::{ScriptValue, UiScript};

fn extended() -> UiScript {
    let mut script = UiScript::new().unwrap();
    // A VM with no host keeps its calls, which the tests read back.
    script.keep_calls();
    super::install(&script).unwrap();
    load_tooltip_template(&script);
    script
}

/// Declares `TestTooltipTemplate`, the stock template's 30 line pairs (`GameTooltipTemplate.xml`):
/// a tooltip made without it holds no line, as on 1.12.1, and FrameXML's own tooltips inherit it.
pub(crate) fn load_tooltip_template(s: &UiScript) {
    let mut xml = String::from(
        r#"<Ui><GameTooltip name="TestTooltipTemplate" virtual="true"><Layers><Layer level="ARTWORK">"#,
    );
    for i in 1..=30 {
        let font = if i == 1 {
            "GameTooltipHeaderText"
        } else {
            "GameTooltipText"
        };
        for side in ["Left", "Right"] {
            xml.push_str(&format!(
                r#"<FontString name="$parentText{side}{i}" inherits="{font}" hidden="true"/>"#
            ));
        }
    }
    xml.push_str("</Layer></Layers></GameTooltip></Ui>");
    let doc = benilla_ui::framexml::parse(&xml).expect("tooltip template parses");
    let report = benilla_ui::loader::load(s, &doc, &|_| None);
    assert!(report.errors.is_empty(), "template: {:?}", report.errors);
}

#[test]
fn tooltip_hide_callback_and_validation_survive_pfui_replacing_global_error() {
    let mut script = extended();
    script.set_realm_name("Realm");
    script.set_unit(
        "player",
        Some(benilla_ui::script::UnitState {
            exists: true,
            name: Some("Test".into()),
            ..Default::default()
        }),
    );
    script.set_social(script_social_for_note());
    script
        .run(
            r#"
        UIParent=CreateFrame('Frame','UIParent')
        GameTooltip=CreateFrame('GameTooltip','GameTooltip',UIParent,'TestTooltipTemplate')
        local button=CreateFrame('Button')
        button:HookScript('OnLeave',GameTooltip_Hide)
        GameTooltip:Show()
        button:GetScript('OnLeave')(button)
        assert(not GameTooltip:IsShown())

        local printed={}
        error=function(message) printed[#printed+1]=message end
        local ok,message=pcall(function() button:HookScript('OnShow',nil) end)
        assert(not ok and message:find('HookScript requires a function',1,true))
        assert(button:GetScript('OnShow')==nil)
        assert(not pcall(function() hooksecurefunc('MissingCallback',nil) end))
        assert(not pcall(function() button:SetAttribute({},'bad') end))
        assert(not pcall(function() button:SetScript('OnClick',false) end))
        assert(not pcall(function() C_FriendList.SetFriendNotes('Alice',{}) end))
        assert(#printed==0)
    "#,
        )
        .unwrap();
    assert!(script.errors().is_empty(), "{:?}", script.errors());
    UiScript::new()
        .unwrap()
        .run("assert(GameTooltip_Hide==nil)")
        .unwrap();
}

#[test]
fn pfui_colour_strings_reach_native_paint_channels() {
    let script = extended();
    script.run(r#"
      local frame=CreateFrame('Frame')
      local texture=frame:CreateTexture(nil,'BORDER')
      -- The channels are stored as bytes, as the reference stores them: within one step of 1/255.
      local function check(r,g,b) assert(math.abs(r-.4)<.004 and math.abs(g-.25)<.004 and math.abs(b-.1)<.004) end
      texture:SetVertexColor('.4',' .25 ','1e-1','1')
      check(texture:GetVertexColor())
      local text=frame:CreateFontString()
      text:SetTextColor('.4','.25','.1')
      check(text:GetTextColor())
      local bar=CreateFrame('StatusBar')
      -- A bar with no texture keeps no colour (`0x782dd0`).
      bar:SetStatusBarTexture('Interface\\TargetingFrame\\UI-StatusBar')
      bar:SetStatusBarColor('.4','.25','.1')
      check(bar:GetStatusBarColor())
      texture:SetGradient('HORIZONTAL','.4','.25','.1','.4','.25','.1')
      texture:SetGradientAlpha('VERTICAL','.4','.25','.1','1','.4','.25','.1','1')
      texture:SetVertexColor(nil,{},'not a number')
      local r,g,b=texture:GetVertexColor(); assert(r==0 and g==0 and b==0)
    "#).unwrap();
    assert!(script.errors().is_empty());
}

#[test]
fn inventory_event_feed_is_idle_before_the_optional_adapter_is_installed() {
    let script = UiScript::new().unwrap();
    benilla_ui::script::pfui_host::items::dispatch_inventory_events(script.lua());
    assert!(script.errors().is_empty());
    assert!(script
        .eval::<bool>("return C_EquipmentSet==nil and C_Item==nil")
        .unwrap());
}

#[test]
fn spell_tooltip_subjects_follow_builders_and_late_views_then_clear() {
    use benilla_ui::script::SpellTooltipView;
    let mut script = extended();
    script.set_spell_tooltip(
        133,
        SpellTooltipView {
            name: "Fireball".into(),
            rank: Some("Rank 1".into()),
            ..Default::default()
        },
    );
    script
        .run(
            r#"
      UIParent=CreateFrame('Frame','UIParent')
      tip=CreateFrame('GameTooltip','SpellTip',UIParent,'TestTooltipTemplate')
      tip:SetOwner(UIParent,'ANCHOR_NONE')
      tip:SetScript('OnTooltipCleared',function(self) assert(not self:HasSpell()) end)
      tip:SetScript('OnShow',function(self) assert(self:HasSpell() and not self:HasItem()) end)
      tip:SetSpellByID(133)
      local name,rank,id=tip:GetSpell()
      assert(tip:HasSpell() and name=='Fireball' and rank=='Rank 1' and id==133)
      tip:Hide(); assert(not tip:HasSpell() and select('#',tip:GetSpell())==0)
      -- Hide un-owned the plate; a builder shows nothing on a plate with no owner.
      tip:SetOwner(UIParent,'ANCHOR_NONE')
      tip:SetSpellByID(143); assert(not tip:HasSpell())
    "#,
        )
        .unwrap();
    script.set_spell_tooltip(
        143,
        SpellTooltipView {
            name: "Fireball".into(),
            ..Default::default()
        },
    );
    script
        .run(
            r#"
      local name,rank,id=tip:GetSpell()
      assert(tip:HasSpell() and name=='Fireball' and rank=='' and id==143)
      tip:ClearLines(); assert(not tip:HasSpell() and not tip:HasItem())
    "#,
        )
        .unwrap();
    assert!(script.errors().is_empty(), "{:?}", script.errors());
}

#[test]
fn equipment_tooltips_count_real_locations_and_retain_missing_item_names() {
    use benilla_ui::script::{pfui_host::items, UnitState, EQUIPMENT_BAG};
    let mut script = extended();
    script.set_realm_name("Realm");
    script.set_unit(
        "player",
        Some(UnitState {
            exists: true,
            name: Some("Test".into()),
            ..Default::default()
        }),
    );
    items::set_instances(
        script.lua(),
        [(
            (EQUIPMENT_BAG, 1),
            items::Instance {
                guid: 100,
                item_id: 123,
                count: 1,
                uses: 1,
                name: Some("Lucky Hat".into()),
            },
        )],
    );
    script.run(r#"
      local id=C_EquipmentSet.CreateEquipmentSet('Hat Set')
      assert(id)
      UIParent=CreateFrame('Frame','UIParent')
      tip=CreateFrame('GameTooltip','EquipmentTip',UIParent,'TestTooltipTemplate')
      tip:SetOwner(UIParent,'ANCHOR_NONE')
      tip:SetEquipmentSet('Hat Set')
      assert(EquipmentTipTextLeft1:GetText()=='Hat Set' and EquipmentTipTextLeft2:GetText()=='1 items')
      assert(EquipmentTipTextLeft3:GetText()=='1 equipped')
      assert(not tip:HasItem() and not tip:HasSpell())
    "#).unwrap();
    items::set_instances(script.lua(), []);
    script
        .run(
            r#"
      tip:SetEquipmentSet('Hat Set')
      assert(EquipmentTipTextLeft3:GetText()=='Missing: Lucky Hat')
      tip:SetEquipmentSet('unknown'); assert(EquipmentTipTextLeft1:GetText()=='Hat Set')
    "#,
        )
        .unwrap();
    assert!(script.errors().is_empty(), "{:?}", script.errors());
}

#[test]
fn inventory_events_coalesce_bags_and_compare_equipment_guids_after_bulk_sync() {
    use benilla_ui::script::{pfui_host::items, UnitGuids, EQUIPMENT_BAG};
    let mut script = extended();
    script.set_unit_guids(&UnitGuids {
        player: 3,
        ..Default::default()
    });
    let item = |guid| items::Instance {
        guid,
        item_id: 25,
        count: 1,
        uses: 1,
        name: None,
    };
    let worn = |guid| {
        let mut slots = [0; 19];
        slots[15] = guid;
        slots
    };
    script.run(r#"
      bagEvents=0; equipmentEvents={}; order={}
      local f=CreateFrame('Frame'); f:RegisterEvent('BAG_UPDATE'); f:RegisterEvent('BAG_UPDATE_DELAYED'); f:RegisterEvent('PLAYER_EQUIPMENT_CHANGED')
      f:SetScript('OnEvent',function(self,evt,slot,present)
        order[#order+1]=evt
        if evt=='BAG_UPDATE_DELAYED' then bagEvents=bagEvents+1; assert(slot==nil)
        elseif evt=='PLAYER_EQUIPMENT_CHANGED' then equipmentEvents[#equipmentEvents+1]={slot,present}; assert(arg1==slot and arg2==present) end
      end)
    "#).unwrap();
    items::set_instances(script.lua(), [((EQUIPMENT_BAG, 16), item(100))]);
    items::observe_equipment_guids(script.lua(), worn(100));
    items::dispatch_inventory_events(script.lua());
    script.run("assert(#equipmentEvents==0)").unwrap();
    script.fire_event("BAG_UPDATE", vec![ScriptValue::Int(0)]);
    script.fire_event("BAG_UPDATE", vec![ScriptValue::Int(1)]);
    script.run("assert(bagEvents==0)").unwrap();
    // The same item entry on a different instance is still a paperdoll change.
    items::observe_equipment_guids(script.lua(), worn(101));
    items::dispatch_inventory_events(script.lua());
    script.run(r#"
      assert(bagEvents==1 and order[1]=='BAG_UPDATE' and order[2]=='BAG_UPDATE' and order[4]=='BAG_UPDATE_DELAYED')
      assert(#equipmentEvents==1 and equipmentEvents[1][1]==16 and equipmentEvents[1][2]==1)
    "#).unwrap();
    items::dispatch_inventory_events(script.lua());
    script
        .run("assert(bagEvents==1 and #equipmentEvents==1)")
        .unwrap();
    items::set_instances(script.lua(), []);
    items::dispatch_inventory_events(script.lua());
    script.run("assert(#equipmentEvents==1)").unwrap();
    items::observe_equipment_guids(script.lua(), worn(0));
    items::dispatch_inventory_events(script.lua());
    script.run("assert(#equipmentEvents==2 and equipmentEvents[2][1]==16 and equipmentEvents[2][2]==nil)").unwrap();
    // A paced multi-merge must not start pfUI's placement phase on its first partial ACK.
    items::set_bag_busy(script.lua(), true);
    script.fire_event("BAG_UPDATE", vec![ScriptValue::Int(0)]);
    items::dispatch_inventory_events(script.lua());
    script.fire_event("BAG_UPDATE", vec![ScriptValue::Int(0)]);
    items::dispatch_inventory_events(script.lua());
    script.run("assert(bagEvents==1)").unwrap();
    items::set_bag_busy(script.lua(), false);
    items::dispatch_inventory_events(script.lua());
    items::dispatch_inventory_events(script.lua());
    script.run("assert(bagEvents==2)").unwrap();
    assert!(script.errors().is_empty(), "{:?}", script.errors());
}

#[test]
fn afk_and_dnd_read_player_flags_for_stock_and_extended_tokens() {
    use benilla_ui::script::{pfui_host::units, UnitGuids, UnitState};
    let mut script = extended();
    script.set_unit_guids(&UnitGuids {
        player: 3,
        ..Default::default()
    });
    let player = UnitState {
        guid: 3,
        exists: true,
        has_object: true,
        is_player: true,
        player_flags: 2,
        ..Default::default()
    };
    script.set_unit("player", Some(player.clone()));
    units::set_unit_states(script.lua(), [(3, player, 0)]);
    script
        .run(
            r#"
      FocusUnit('player')
      assert(UnitIsAFK('player') and UnitIsAFK('focus') and not UnitIsDND('focus'))
      assert(not UnitIsAFK('pet') and not UnitIsDND('pet'))
    "#,
        )
        .unwrap();
    let player = UnitState {
        guid: 3,
        exists: true,
        has_object: true,
        is_player: true,
        player_flags: 4,
        ..Default::default()
    };
    script.set_unit("player", Some(player.clone()));
    units::set_unit_states(script.lua(), [(3, player, 0)]);
    script
        .run("assert(not UnitIsAFK('focus') and UnitIsDND('focus'))")
        .unwrap();
}

/// `quests` as `(id, header, title)`, seated in slots 0 on with their records held and the
/// player in the world, as the app feeds the quest log: one header row per header, its quests
/// under it in slot order.
fn seat_quest_log(script: &mut UiScript, quests: &[(u32, &str, &str)]) {
    use benilla_ui::script::{QuestRecordView, QuestSlotView, QUEST_LOG_SLOTS};
    let mut slots = [QuestSlotView::default(); QUEST_LOG_SLOTS];
    for (i, &(id, header, title)) in quests.iter().enumerate() {
        script.seed_quest_record(
            id,
            QuestRecordView {
                title: title.into(),
                level: 1,
                zone_or_sort: 1,
                header: header.into(),
                ..Default::default()
            },
        );
        slots[i] = QuestSlotView {
            quest_id: id,
            ..Default::default()
        };
    }
    script.set_quest_log_player(true);
    script.set_quest_slots(slots);
    script.quest_log_changed();
}

#[test]
fn quest_events_ignore_bulk_sync_wait_for_templates_and_keep_turnin_before_removal() {
    use benilla_ui::script::pfui_host::quests;
    let mut script = extended();
    script.run(r#"
      events={}
      local f=CreateFrame('Frame')
      f:RegisterEvent('QUEST_ACCEPTED'); f:RegisterEvent('QUEST_REMOVED'); f:RegisterEvent('QUEST_TURNED_IN')
      f:SetScript('OnEvent',function(self,evt,a,b,c)
        assert(this==self and event==evt and arg1==a and arg2==b and arg3==c)
        if evt=='QUEST_ACCEPTED' and a>0 then assert(C_QuestLog.GetQuestIDForLogIndex(a)==b) end
        events[#events+1]={evt,a,b,c}
      end)
    "#).unwrap();
    quests::observe_log_slots(script.lua(), vec![(7, 0), (0, 0)]);
    quests::dispatch_log_events(script.lua());
    script.run("assert(#events==0)").unwrap();
    quests::observe_log_slots(script.lua(), vec![(7, 0), (783, 0)]);
    quests::dispatch_log_events(script.lua());
    script.run("assert(#events==0)").unwrap();
    // Northshire with 783 in slot 0 (row 2) and the bulk-synced 7 in slot 1.
    seat_quest_log(
        &mut script,
        &[(783, "Northshire", ""), (7, "Northshire", "")],
    );
    quests::dispatch_log_events(script.lua());
    // The accept names the log row the quest sits on as the rebuild ordered it.
    script.run("assert(#events==1 and events[1][1]=='QUEST_ACCEPTED' and C_QuestLog.GetQuestIDForLogIndex(events[1][2])==783 and events[1][3]==783)").unwrap();
    // Header folds and raw-slot reordering are presentation changes, never accepts/removals.
    quests::observe_log_slots(script.lua(), vec![(783, 0), (7, 0)]);
    quests::dispatch_log_events(script.lua());
    script.run("assert(#events==1)").unwrap();
    quests::turned_in(script.lua(), 783, 40, 12);
    quests::observe_log_slots(script.lua(), vec![(0, 0), (7, 0)]);
    quests::dispatch_log_events(script.lua());
    script.run(r#"
      assert(#events==3 and events[2][1]=='QUEST_TURNED_IN' and events[2][2]==783 and events[2][3]==40 and events[2][4]==12)
      assert(events[3][1]=='QUEST_REMOVED' and events[3][2]==783)
    "#).unwrap();
    // A quest accepted and removed while its template is cold must never surface later.
    quests::observe_log_slots(script.lua(), vec![(99, 0), (7, 0)]);
    quests::observe_log_slots(script.lua(), vec![(0, 0), (7, 0)]);
    quests::dispatch_log_events(script.lua());
    script
        .run("assert(#events==4 and events[4][1]=='QUEST_REMOVED' and events[4][2]==99)")
        .unwrap();
    assert!(script.errors().is_empty(), "{:?}", script.errors());
}

#[test]
fn run_next_frame_waits_for_the_frame_after_the_one_that_asked() {
    let mut script = extended();
    script
        .run(
            r#"
      ran=0
      local f=CreateFrame('Frame'); f:RegisterEvent('PLAYER_ENTERING_WORLD')
      f:SetScript('OnEvent',function() RunNextFrame(function() ran=ran+1 end) end)
    "#,
        )
        .unwrap();
    script.fire_event("PLAYER_ENTERING_WORLD", vec![]);
    script.tick(0.01);
    assert_eq!(
        script.eval::<i64>("return ran").unwrap(),
        0,
        "not in the event's own frame"
    );
    script.tick(0.01);
    assert_eq!(script.eval::<i64>("return ran").unwrap(), 1);
    script.tick(0.01);
    assert_eq!(script.eval::<i64>("return ran").unwrap(), 1, "once");
}

#[test]
fn chat_bubbles_list_the_engines_own_frames() {
    use benilla_ui::script::{BubbleGeometry, BubbleState};
    let mut script = extended();
    script
        .run(
            r#"
      WorldFrame=CreateFrame('Frame','WorldFrame'); WorldFrame:SetWidth(1024); WorldFrame:SetHeight(768)
      local other=CreateFrame('Frame',nil,WorldFrame)
      other:CreateTexture():SetTexture('Interface\\Buttons\\WHITE8X8')
      assert(#C_ChatBubbles.GetAllChatBubbles()==0)
    "#,
        )
        .unwrap();
    let geometry = BubbleGeometry {
        border: 16.0,
        margin: 16.0,
        wrap_cap: 300.0,
        text_height: 14.0,
        device_px: 1.0,
    };
    let line = |key, text: &str, shown| BubbleState {
        key,
        line: key,
        text: text.into(),
        colour: [1.0, 1.0, 1.0],
        shown,
        alpha: 255,
        seat: (400.0, 300.0),
    };
    script.sync_chat_bubbles(
        geometry,
        &[line(1, "First line", true), line(2, "Out of sight", false)],
    );
    script
        .run(
            r#"
      local list=C_ChatBubbles.GetAllChatBubbles()
      assert(#list==2, 'both bubbles, the hidden one too, and no other child')
      local texts={}
      for _,bubble in ipairs(list) do
        for _,region in ipairs({bubble:GetRegions()}) do
          if region:GetObjectType()=='FontString' then texts[region:GetText()]=bubble:IsShown() end
        end
      end
      assert(texts['First line']==1 and texts['Out of sight']==nil)
      list[1]=nil; assert(#C_ChatBubbles.GetAllChatBubbles()==2, 'each call answers a fresh list')
    "#,
        )
        .unwrap();
    script.sync_chat_bubbles(geometry, &[]);
    script
        .run("for _,bubble in ipairs(C_ChatBubbles.GetAllChatBubbles()) do assert(not bubble:IsShown()) end")
        .unwrap();
    assert!(script.errors().is_empty(), "{:?}", script.errors());
}

#[test]
fn tooltip_subjects_follow_real_builders_clear_callbacks_and_retargeting() {
    use benilla_ui::script::{
        ContainerSlot, ContainerState, ItemTemplateView, UnitGuids, UnitState,
    };
    use std::collections::HashMap;
    let mut script = extended();
    script.set_item_template(
        38,
        ItemTemplateView {
            name: "Shirt".into(),
            quality: 1,
            ..Default::default()
        },
    );
    script.set_container(
        0,
        Some(ContainerState {
            num_slots: 2,
            slots: HashMap::from([
                (
                    1,
                    ContainerSlot {
                        item_id: 38,
                        link: Some("|cffffffff|Hitem:38:2:3:4|h[Shirt of Testing]|h|r".into()),
                        count: 1,
                        ..Default::default()
                    },
                ),
                (
                    2,
                    ContainerSlot {
                        item_id: 99999,
                        link: Some("|cffffffff|Hitem:99999:0:0:0|h[Cold item]|h|r".into()),
                        count: 1,
                        ..Default::default()
                    },
                ),
            ]),
            ..Default::default()
        }),
    );
    script.set_unit_guids(&UnitGuids {
        player: 1,
        target: 5,
        party: [5, 0, 0, 0],
        held: HashMap::from([(1, 0), (5, 0)]),
        ..Default::default()
    });
    script.set_unit(
        "target",
        Some(UnitState {
            guid: 5,
            name: Some("Test NPC".into()),
            exists: true,
            has_object: true,
            health: 20,
            max_health: 20,
            ..Default::default()
        }),
    );
    script.run(r#"
      UIParent=CreateFrame('Frame','UIParent')
      tip=CreateFrame('GameTooltip','ProbeTooltip',UIParent,'TestTooltipTemplate')
      tip:SetOwner(UIParent,'ANCHOR_NONE')
      tip:SetScript('OnTooltipCleared',function(self) assert(not self:HasItem()) end)
      tip:SetScript('OnShow',function(self) assert(self:HasItem()) end)
      tip:SetBagItem(0,1)
      local name,link,id=tip:GetItem()
      assert(name=='Shirt of Testing' and id==38 and link=='|cffffffff|Hitem:38:2:3:4|h[Shirt of Testing]|h|r')
      assert(tip:HasItem() and not tip:HasUnit())
      tip:SetBagItem(0,9); assert(select(3,tip:GetItem())==38)
      tip:SetBagItem(0,2); assert(select(3,tip:GetItem())==99999)
      tip:Hide(); assert(not tip:HasItem() and tip:GetItem()==nil)
      tip:SetScript('OnShow',nil); tip:SetOwner(UIParent,'ANCHOR_NONE'); tip:SetUnit('target')
      assert(tip:HasUnit() and not tip:HasItem())
      local name,guid=tip:GetUnitGUID()
      assert(name=='Test NPC' and guid=='0x0000000000000005')
      assert(UnitTokenFromGUID(guid)=='party1' and UnitTokenFromGUID('bad')==nil and UnitTokenFromGUID('0x0000000000000000')==nil)
    "#).unwrap();
    script.set_unit_guids(&UnitGuids {
        player: 1,
        held: HashMap::from([(1, 0), (5, 0)]),
        ..Default::default()
    });
    benilla_ui::script::pfui_host::units::set_unit_states(
        script.lua(),
        [(
            5,
            UnitState {
                guid: 5,
                name: Some("Test NPC".into()),
                exists: true,
                has_object: true,
                health: 20,
                max_health: 20,
                ..Default::default()
            },
            0,
        )],
    );
    script.run(r#"
      assert(select(2,tip:GetUnitGUID())=='0x0000000000000005' and UnitTokenFromGUID('0x0000000000000005')==nil)
      FocusUnit('0x0000000000000005')
      tip:SetUnit('focus'); assert(tip:HasUnit() and select(2,tip:GetUnitGUID())==UnitGUID('focus'))
      assert(UnitGUID('npc')==nil and UnitGUID('mouseover')==nil)
      tip:SetUnit('0x0000000000000005'); assert(tip:HasUnit())
      tip:ClearLines(); assert(not tip:HasUnit() and tip:GetUnitGUID()==nil)
    "#).unwrap();
    assert!(script.errors().is_empty(), "{:?}", script.errors());
}

#[test]
fn map_overlays_follow_the_viewed_zone_and_return_independent_real_tile_tables() {
    use benilla_ui::script::{WorldMapContinentView, WorldMapZoneView};
    let mut script = extended();
    let overlay = super::maps::Overlay {
        name: "TEST".into(),
        path: "Interface\\WorldMap\\Zone\\TEST".into(),
        areas: vec![123, 124],
        size: [256, 256],
        grid: [1, 1],
        tiles: vec![super::maps::Tile {
            file: "Interface\\WorldMap\\Zone\\TEST1".into(),
            width: 256.0,
            height: 256.0,
            uv: [1.0; 2],
            offset: [10.0, 20.0],
        }],
        ..Default::default()
    };
    let mut catalog = super::maps::Catalog::default();
    catalog.zones = std::collections::BTreeMap::from([(12, vec![overlay]), (85, vec![])]);
    catalog.install(script.lua()).unwrap();
    script.set_world_map_catalog(vec![WorldMapContinentView {
        zones: vec![
            WorldMapZoneView {
                area_id: 12,
                map_file: "Zone".into(),
                ..Default::default()
            },
            WorldMapZoneView {
                area_id: 85,
                map_file: "Another".into(),
                ..Default::default()
            },
        ],
        ..Default::default()
    }]);
    script.run(r#"
      assert(C_Map.GetMapOverlays()==nil and C_Map.GetMapOverlays(999999)==nil)
      SetMapZoom(1,1)
      local list=C_Map.GetMapOverlays(); assert(#list==1 and list[1].areaIDs[2]==124)
      assert(list[1].tiles[1].file=='Interface\\WorldMap\\Zone\\TEST1' and list[1].tiles[1].offsetX==10)
      list[1].tiles[1].width=0; assert(C_Map.GetMapOverlays()[1].tiles[1].width==256)
      SetMapZoom(1,2); assert(#C_Map.GetMapOverlays()==0 and #C_Map.GetMapOverlays(12)==1)
      SetMapZoom(0); assert(C_Map.GetMapOverlays()==nil)
    "#).unwrap();
    assert!(script.errors().is_empty(), "{:?}", script.errors());
}

#[test]
fn global_event_registry_routes_real_events_and_preserves_nil_payloads_and_owners() {
    let mut script = extended();
    script.run(r#"
      calls=0; owner={}
      EventRegistry:RegisterFrameEventAndCallback('BAG_NEW_ITEMS_UPDATED',function(self, extra, bag, hole, slot)
        assert(self==owner and extra=='extra' and bag==0 and hole==nil and slot==2)
        calls=calls+1
      end,owner,'extra')
    "#).unwrap();
    script.fire_event(
        "BAG_NEW_ITEMS_UPDATED",
        vec![ScriptValue::Int(0), ScriptValue::Nil, ScriptValue::Int(2)],
    );
    script.run("assert(calls==1); EventRegistry:UnregisterFrameEventAndCallback('BAG_NEW_ITEMS_UPDATED',owner)").unwrap();
    script.fire_event("BAG_NEW_ITEMS_UPDATED", vec![]);
    script.run("assert(calls==1)").unwrap();
    assert!(script.errors().is_empty(), "{:?}", script.errors());
}

#[test]
fn cursor_info_reads_real_pickups_without_replacing_stock_cursor_state() {
    use benilla_ui::script::{ContainerSlot, ContainerState, SpellBookState, SpellSlotView};
    let mut script = extended();
    script.set_spellbook(SpellBookState {
        slots: vec![SpellSlotView {
            spell_id: 133,
            name: "Fireball".into(),
            ..Default::default()
        }],
        ..Default::default()
    });
    script.set_container(
        0,
        Some(ContainerState {
            num_slots: 1,
            slots: std::collections::HashMap::from([(
                1,
                ContainerSlot {
                    item_id: 38,
                    link: Some("item:38:2:3:4".into()),
                    count: 1,
                    ..Default::default()
                },
            )]),
            ..Default::default()
        }),
    );
    script.run(r#"
      assert(GetCursorInfo()==nil)
      PickupSpell(1,'spell')
      local kind,slot,book,id=GetCursorInfo()
      assert(kind=='spell' and slot==1 and book=='spell' and id==133 and CursorHasSpell())
      ClearCursor(); PickupContainerItem(0,1)
      local kind,id,link=GetCursorInfo()
      assert(kind=='item' and id==38 and link=='item:38:2:3:4' and CursorHasItem())
      ClearCursor(); assert(GetCursorInfo()==nil and not CursorHasItem())
      local macro=C_Macro.CreateMacro('Probe','INV_Sword','/say test',false)
      PickupMacro(macro)
      local kind,id=GetCursorInfo(); assert(kind=='macro' and id==macro)
      ClearCursor(); DeleteMacro(macro)
      assert(INVSLOTS_EQUIPABLE_IN_COMBAT[16] and INVSLOTS_EQUIPABLE_IN_COMBAT[17] and INVSLOTS_EQUIPABLE_IN_COMBAT[18])
      assert(not INVSLOTS_EQUIPABLE_IN_COMBAT[5] and HEARTHSTONE_ITEM_ID==6948)
    "#).unwrap();
    assert!(script.errors().is_empty(), "{:?}", script.errors());
}

#[test]
fn runtime_spell_queries_resolve_ids_links_names_and_preserve_cold_held_and_expired_timers() {
    use super::spell::{
        runtime::{self, Cost},
        Catalog,
    };
    use benilla_ui::script::{
        pfui_host::units, SpellBookState, SpellSlotView, UnitGuids, UnitState,
    };
    use std::collections::HashMap;
    let mut script = extended();
    Catalog(Default::default()).install(script.lua()).unwrap();
    script.set_spellbook(SpellBookState {
        slots: vec![
            SpellSlotView {
                spell_id: 133,
                name: "Fireball".into(),
                rank: Some("Rank 1".into()),
                ..Default::default()
            },
            SpellSlotView {
                spell_id: 143,
                name: "Fireball".into(),
                rank: Some("Rank 2".into()),
                ..Default::default()
            },
        ],
        ..Default::default()
    });
    script.set_unit_guids(&UnitGuids {
        player: 1,
        target: 5,
        ..Default::default()
    });
    units::set_unit_states(
        script.lua(),
        [(
            5,
            UnitState {
                guid: 5,
                exists: true,
                has_object: true,
                ..Default::default()
            },
            0,
        )],
    );
    runtime::set_costs(
        script.lua(),
        HashMap::from([
            (
                133,
                Cost {
                    kind: 0,
                    amount: 20,
                    percent: 0,
                },
            ),
            (
                143,
                Cost {
                    kind: 0,
                    amount: 40,
                    percent: 2,
                },
            ),
            (
                144,
                Cost {
                    kind: 3,
                    amount: 0,
                    percent: 0,
                },
            ),
        ]),
    );
    runtime::set_cooldowns(
        script.lua(),
        HashMap::from([
            (133, (0, 0, true)),
            (143, (0, 5000, true)),
            (144, (0, 1000, false)),
        ]),
    );
    runtime::set_usable(
        script.lua(),
        HashMap::from([(133, (true, false)), (143, (false, true))]),
    );
    runtime::set_range_query(
        script.lua(),
        Some(Box::new(|id, guid| (guid == 5).then_some(id == 133))),
    );
    script.run(r#"
      local cost=C_Spell.GetSpellPowerCost('Fireball')[1]
      assert(cost.cost==40 and cost.minCost==40 and cost.costPercent==2 and cost.name=='MANA' and cost.type==0)
      assert(C_Spell.GetSpellPowerCost('Fireball(Rank 1)')[1].cost==20)
      assert(C_Spell.GetSpellPowerCost('|cfff00|Hspell:133|h[Fireball]|h|r')[1].cost==20)
      assert(C_Spell.GetSpellPowerCost(999)==nil and C_Spell.GetSpellPowerCost(144)==nil)
      local cd=C_Spell.GetSpellCooldown('spell:133')
      assert(cd.startTime==0 and cd.duration==0 and cd.isEnabled and cd.modRate==1 and cd.isOnGCD==nil)
      assert(C_Spell.GetSpellCooldown(143).duration==5 and not C_Spell.GetSpellCooldown(144).isEnabled)
      local usable,oom=C_Spell.IsSpellUsable(133); assert(usable and not oom)
      local usable,oom=C_Spell.IsSpellUsable('Fireball'); assert(not usable and oom)
      local usable,oom=C_Spell.IsSpellUsable(999); assert(not usable and not oom)
      FocusUnit('target')
      assert(C_Spell.IsSpellInRange('spell:133','focus')==true and C_Spell.IsSpellInRange(143,'target')==false)
      assert(C_Spell.IsSpellInRange(133,'pet')==nil)
      assert(not pcall(C_Spell.IsSpellInRange,133,'garbage'))
    "#).unwrap();
    script.tick(6.0);
    script.run("assert(C_Spell.GetSpellCooldown(143).duration==0 and C_Spell.GetSpellCooldown(144).duration==1)").unwrap();
    assert!(script.errors().is_empty(), "{:?}", script.errors());
}

#[test]
fn loot_history_preserves_roll_identity_results_and_winner_without_a_prior_vote() {
    use super::loot::{self, Event, Key, Player};
    let script = extended();
    script.run(r#"
      changed=0; complete=0; full=0
      local f=CreateFrame('Frame')
      f:RegisterEvent('LOOT_HISTORY_FULL_UPDATE'); f:RegisterEvent('LOOT_HISTORY_ROLL_CHANGED'); f:RegisterEvent('LOOT_HISTORY_ROLL_COMPLETE')
      f:SetScript('OnEvent',function(self,event,item,player)
        if event=='LOOT_HISTORY_FULL_UPDATE' then full=full+1
        elseif event=='LOOT_HISTORY_ROLL_CHANGED' then assert(item==1); changed=changed+1
        else assert(item==1); complete=complete+1 end
      end)
    "#).unwrap();
    let key = Key { target: 7, slot: 3 };
    loot::observe(script.lua(), key, 123, -5, 2., Event::Start);
    loot::observe(script.lua(), key, 123, -5, 3., Event::Start);
    let mut player = Player {
        guid: 3,
        name: Some("Player".into()),
        class: Some("WARRIOR".into()),
        kind: 1,
        roll: 0,
        is_me: true,
    };
    loot::observe(script.lua(), key, 123, -5, 3., Event::Roll(player.clone()));
    player.name = None;
    player.class = None;
    player.roll = 42;
    loot::observe(script.lua(), key, 123, -5, 4., Event::Roll(player));
    loot::observe(
        script.lua(),
        key,
        123,
        -5,
        5.,
        Event::Won(Player {
            guid: 8,
            name: Some("Winner".into()),
            class: Some("MAGE".into()),
            kind: 1,
            roll: 98,
            is_me: false,
        }),
    );
    loot::observe(
        script.lua(),
        key,
        123,
        -5,
        5.5,
        Event::Roll(Player {
            guid: 3,
            name: None,
            class: None,
            kind: 1,
            roll: 0,
            is_me: true,
        }),
    );
    loot::observe(script.lua(), key, 123, -5, 5.5, Event::Passed);
    script
        .run(
            r#"
      local id,link,players,done,winner,time=C_LootHistory.GetItem(1)
      assert(id==1 and link=='item:123:0:-5:0' and players==2 and done and winner==2 and time==2)
      local name,class,kind,roll,won,me=C_LootHistory.GetPlayerInfo(1,1)
      assert(name=='Player' and class=='WARRIOR' and kind==1 and roll==42 and not won and me)
      assert(C_LootHistory.GetPlayerInfo(1,2)=='Winner' and C_LootHistory.GetItem(2)==nil)
      assert(C_LootHistory.GetItem(-1)==nil and C_LootHistory.GetPlayerInfo(1,0)==nil)
      assert(full==1 and changed==4 and complete==1)
      C_LootHistory.Clear(); assert(full==2 and C_LootHistory.GetNumItems()==0)
    "#,
        )
        .unwrap();
    loot::observe(script.lua(), key, 123, -5, 6., Event::Passed);
    script.run("local id,_,_,done,winner=C_LootHistory.GetItem(1); assert(id==2 and done and winner==nil)").unwrap();
    assert!(script.errors().is_empty(), "{:?}", script.errors());
}

#[test]
fn loot_ring_eviction_changes_indices_without_changing_roll_ids_and_reload_resets() {
    use super::loot::{self, Event, Key};
    let script = extended();
    for slot in 1..=130 {
        loot::observe(
            script.lua(),
            Key { target: 5, slot },
            100,
            0,
            slot as f64,
            Event::Start,
        );
    }
    script.run("assert(C_LootHistory.GetNumItems()==128 and C_LootHistory.GetItem(1)==3 and C_LootHistory.GetItem(128)==130)").unwrap();
    extended()
        .run("assert(C_LootHistory.GetNumItems()==0)")
        .unwrap();
}

#[test]
fn focus_power_events_report_only_the_resource_that_changed() {
    use benilla_ui::script::{
        pfui_host::units::{self, Power},
        UnitGuids, UnitState,
    };
    let mut script = extended();
    script.set_unit_guids(&UnitGuids {
        target: 5,
        ..Default::default()
    });
    units::set_unit_states(
        script.lua(),
        [(
            5,
            UnitState {
                guid: 5,
                exists: true,
                has_object: true,
                ..Default::default()
            },
            0,
        )],
    );
    let mut power = Power {
        kind: 1,
        current: [Some(100), Some(200), Some(0), Some(50), Some(0)],
        maximum: [Some(500), Some(1000), Some(0), Some(100), Some(0)],
    };
    units::set_powers(script.lua(), [(5, power.clone())]);
    script.run(r#"
      FocusUnit('target'); resourceEvents={}
      local f=CreateFrame('Frame')
      for _,event in ipairs({'UNIT_MANA','UNIT_RAGE','UNIT_ENERGY','UNIT_DISPLAYPOWER','UNIT_POWER_UPDATE'}) do f:RegisterUnitEvent(event,'focus') end
      f:SetScript('OnEvent',function(self,event,unit,kind)
        assert(unit=='focus'); table.insert(resourceEvents,event..':'..tostring(kind))
      end)
    "#).unwrap();
    power.current[1] = Some(250);
    units::set_powers(script.lua(), [(5, power.clone())]);
    script.run("assert(#resourceEvents==2 and resourceEvents[1]=='UNIT_RAGE:nil' and resourceEvents[2]=='UNIT_POWER_UPDATE:RAGE' and UnitPower('focus')==25)").unwrap();
    units::set_powers(script.lua(), [(5, power.clone())]);
    script.run("assert(#resourceEvents==2)").unwrap();
    // ClassicAPI's generic power events reach the stock token too; the engine's own stay its.
    script.run(r#"
      targetEvents={}
      local t=CreateFrame('Frame')
      for _,event in ipairs({'UNIT_RAGE','UNIT_POWER_UPDATE','UNIT_MAXPOWER'}) do t:RegisterUnitEvent(event,'target') end
      t:SetScript('OnEvent',function(self,event,unit,kind) table.insert(targetEvents,event..':'..tostring(kind)) end)
    "#).unwrap();
    power.current[1] = Some(300);
    power.maximum[1] = Some(1100);
    units::set_powers(script.lua(), [(5, power)]);
    script.run("assert(table.concat(targetEvents,' ')=='UNIT_POWER_UPDATE:RAGE UNIT_MAXPOWER:RAGE', table.concat(targetEvents,' '))").unwrap();
    // ClassicAPI's types (unit/Power.cpp): happiness is 27 and shown over 1.12's divisor of 1000;
    // a type other than the display power answers 0 for a unit that is neither player nor pet;
    // type 4 is the player's combo points, 0 for any other unit.
    let mut pet = Power {
        kind: 2,
        current: [Some(0), Some(0), Some(80), Some(0), Some(612_000)],
        maximum: [Some(0), Some(0), Some(100), Some(0), Some(1_050_000)],
    };
    units::set_powers(script.lua(), [(5, pet.clone())]);
    script
        .run(
            r#"
      assert(Enum.PowerType.Happiness == 27 and Enum.PowerType.ComboPoints == 4)
      assert(UnitPower('target') == 80 and UnitPowerMax('target', 27) == 1050)
      assert(UnitPower('target', 27) == 0, 'not the display power, not the pet: gated')
      assert(UnitPower('target', Enum.PowerType.ComboPoints) == 0 and UnitPower('target', 99) == 0)
    "#,
        )
        .unwrap();
    script.set_unit_guids(&UnitGuids {
        target: 5,
        pet: 5,
        ..Default::default()
    });
    script
        .run("assert(UnitPower('target', 27) == 612 and UnitPower('target', 27, true) == 612000)")
        .unwrap();
    pet.current[0] = Some(30);
    pet.maximum[0] = Some(50);
    units::set_powers(script.lua(), [(5, pet)]);
    script
        .run("assert(UnitPowerMax('target', 0) == 50, 'the max is not gated')")
        .unwrap();
    assert!(script.errors().is_empty(), "{:?}", script.errors());
}

#[test]
fn icon_catalog_and_picker_preserve_sources_filters_and_saved_paths() {
    let script = extended();
    let catalog = super::icons::Catalog::from_paths(
        [
            "Interface/Icons/Spell_Fire.blp",
            "interface\\icons\\SPELL_FIRE.tga",
            "Interface/Icons/Ability_Kick.blp",
            "Interface/Icons/INV_Sword.blp",
            "Interface/Icons/Subdir/INV_Other.blp",
            "Interface/Icons/INV_Note.txt",
        ]
        .map(str::to_owned),
    )
    .with_loose_paths(["Interface/Icons/INV_Sword.tga".to_owned()]);
    catalog.feed(script.lua());
    script.run(r#"
      local spells={'keep'}; GetMacroIcons(spells)
      assert(#spells==3 and spells[1]=='keep' and spells[2]=='ABILITY_KICK' and spells[3]=='SPELL_FIRE')
      local items={}; GetLooseMacroItemIcons(items); GetMacroItemIcons(items)
      assert(#items==2 and items[1]=='INV_SWORD' and items[2]=='INV_SWORD')
      local provider=CreateAndInitFromMixin(IconDataProviderMixin,IconDataProviderExtraType.None)
      assert(provider:GetNumIcons()==5 and provider:GetIconByIndex(2)=='INTERFACE\\ICONS\\ABILITY_KICK')
      provider:SetIconTypes({IconDataProviderIconType.Item})
      assert(provider:GetNumIcons()==3 and provider:GetIconForSaving(2)=='INV_SWORD')
      assert(provider:GetIconByIndex(4)==nil and provider:GetIconByIndex(0)==nil)
      local other=CreateAndInitFromMixin(IconDataProviderMixin,IconDataProviderExtraType.None)
      provider:Release(); provider:Release()
      assert(other:GetNumIcons()==5 and other:GetIndexOfIcon('INTERFACE\\ICONS\\SPELL_FIRE')==3)
      local extra=CreateAndInitFromMixin(IconDataProviderMixin,IconDataProviderExtraType.None,true)
      assert(extra:GetNumIcons()==1 and extra:GetIconByIndex(2)==nil)
    "#).unwrap();
}

#[test]
fn numeric_unit_traits_resolve_focus_markers_and_raw_guids() {
    use benilla_ui::script::{
        pfui_host::units::{self, Traits},
        UnitGuids, UnitState,
    };
    let mut script = extended();
    script.set_unit_guids(&UnitGuids {
        target: 5,
        ..Default::default()
    });
    units::set_traits(
        script.lua(),
        [(
            5,
            Traits {
                creature_type: 11,
                created_by_spell: 3599,
            },
        )],
    );
    units::set_unit_states(
        script.lua(),
        [(
            5,
            UnitState {
                guid: 5,
                exists: true,
                has_object: true,
                raid_target: 2,
                ..Default::default()
            },
            0,
        )],
    );
    script
        .run(
            r#"
      FocusUnit('target'); assert(UnitCreatureTypeID('focus')==11)
      assert(UnitCreatedBySpell('mark2')==3599 and UnitCreatedBySpell('0x00000005')==3599)
      assert(UnitCreatureTypeID('mark1')==nil and UnitCreatedBySpell('targettarget')==nil)
    "#,
        )
        .unwrap();
    units::set_traits(
        script.lua(),
        [(
            5,
            Traits {
                creature_type: 0,
                created_by_spell: 0,
            },
        )],
    );
    script
        .run("assert(UnitCreatureTypeID('focus')==nil and UnitCreatedBySpell('target')==nil)")
        .unwrap();
}

#[test]
fn raw_fields_and_minion_queries_preserve_server_values() {
    use benilla_ui::script::{
        pfui_host::units::{self, RawUnit},
        UnitGuids, UnitState,
    };
    let mut script = extended();
    script.set_unit_guids(&UnitGuids {
        player: 3,
        target: 5,
        ..Default::default()
    });
    units::set_unit_states(
        script.lua(),
        [(
            5,
            UnitState {
                guid: 5,
                exists: true,
                has_object: true,
                flags: 0x2000_0000,
                owner: 3,
                ..Default::default()
            },
            0,
        )],
    );
    let mut auras = vec![0; 48];
    auras[32] = 123;
    let mut applications = vec![0; 48];
    applications[32] = 17;
    units::set_raw_units(
        script.lua(),
        [(
            5,
            RawUnit {
                health: Some(1337),
                form: 0,
                mount_display: 0,
                auras,
                applications,
            },
        )],
    );
    script.run(r#"
        assert(UnitIsFeignDeath('target') and UnitIsMinion('target'))
        assert(not UnitIsMinion('player') and not UnitIsFeignDeath('mark9'))
        assert(GetUnitField('target','health')==1337 and GetPlayerGuid()=='0x0000000000000003')
        local ids=GetUnitField('0x00000005','aura'); local apps=GetUnitField('target','auraApplications')
        assert(#ids==48 and ids[33]==123 and ids[32]==0 and #apps==48 and apps[33]==17)
        assert(GetUnitField('mark9','health')==nil)
    "#).unwrap();
}

/// ClassicAPI's `LOOT_HISTORY_AUTO_SHOW(rollID, nil)` goes once per roll, with the history's id
/// for it (`loot/History.cpp`); a roll not in the history shows nothing.
#[test]
fn loot_history_auto_show_goes_once_per_roll() {
    use crate::loot::{self, Event, Key};
    let script = extended();
    script
        .run(r#"
        shown = {}
        local f = CreateFrame('Frame'); f:RegisterEvent('LOOT_HISTORY_AUTO_SHOW')
        f:SetScript('OnEvent', function() table.insert(shown, tostring(arg1) .. ' ' .. tostring(arg2)) end)
        "#)
        .unwrap();
    let key = Key { target: 9, slot: 2 };
    loot::observe(script.lua(), key, 19019, 0, 1.0, Event::Start);
    loot::auto_show(script.lua(), key);
    loot::auto_show(script.lua(), key);
    loot::auto_show(script.lua(), Key { target: 9, slot: 3 });
    let shown: String = script.eval("return table.concat(shown, ';')").unwrap();
    let id: i64 = script.eval("return (C_LootHistory.GetItem(1))").unwrap();
    assert_eq!(shown, format!("{id} nil"));
}

/// ClassicAPI's `C_ColorUtil.WrapTextInColor` / `WrapTextInColorCode` (`color/Util.cpp`), which its
/// 1.23 addon's `WrapTextInColor` global calls.
#[test]
fn color_util_wraps_text_in_a_color_code() {
    let script = extended();
    script
        .run(
            r#"
        assert(C_ColorUtil.WrapTextInColorCode('Hi', 'ffff0000') == '|cffff0000Hi|r')
        assert(C_ColorUtil.WrapTextInColor('Hi', CreateColor(1, 0, 0)) == '|cffff0000Hi|r')
        "#,
        )
        .unwrap();
}

/// ClassicAPI 1.23's Settings window pieces: `editbox:HasFocus()`, `C_SettingsUtil` firing its two
/// events inside the call, and the soft-interact CVars at Classic Era's defaults
/// (`editbox/Methods.cpp`, `settings/Panel.cpp`, `target/SoftInteract.cpp`).
#[test]
fn settings_window_pieces_answer_as_classicapi() {
    let script = extended();
    script
        .run(r#"
        local box = CreateFrame('EditBox', 'ProbeFocusBox')
        assert(box:HasFocus() == false)
        box:SetFocus(); assert(box:HasFocus() == true)
        box:ClearFocus(); assert(box:HasFocus() == false)
        got = {}
        local f = CreateFrame('Frame'); f:RegisterEvent('SETTINGS_PANEL_OPEN'); f:RegisterEvent('SETTINGS_LOADED')
        f:SetScript('OnEvent', function() table.insert(got, event .. ' ' .. tostring(arg1) .. ' ' .. tostring(arg2)) end)
        C_SettingsUtil.OpenSettingsPanel('pfUI', 'Fonts')
        C_SettingsUtil.NotifySettingsLoaded()
        assert(table.concat(got, ';') == 'SETTINGS_PANEL_OPEN pfUI Fonts;SETTINGS_LOADED nil nil', table.concat(got, ';'))
        assert(GetCVar('SoftTargetInteract') == '0' and GetCVar('SoftTargetInteractRange') == '10')
        assert(GetCVar('TargetPriorityAutoTargetIgnoreWindow') == '500' and GetCVar('countdownForCooldowns') == '0')
        -- ClassicAPI's CVars are the engine's: the addons keep all 30 RegisterCVar slots.
        for i = 1, 30 do RegisterCVar('ProbeAddonCVar' .. i, '1') end
        assert(not pcall(RegisterCVar, 'ProbeAddonCVar31', '1'))
        "#)
        .unwrap();
}

/// ClassicAPI's `FlashClientIcon` returns nothing and leaves one ask for the app, however many
/// calls a frame makes (`screen/Flash.cpp`).
#[test]
fn flash_client_icon_leaves_one_ask() {
    use benilla_ui::script::pfui_host::attention;
    let script = extended();
    assert!(!attention::take(script.lua()));
    script
        .run("assert(select('#', FlashClientIcon()) == 0); FlashClientIcon(true)")
        .unwrap();
    assert!(attention::take(script.lua()));
    assert!(!attention::take(script.lua()));
}

/// ClassicAPI's curves (`curve/Curve.cpp`): ends hold, Linear and Step between points, Cubic a
/// Catmull-Rom from the second point to the one before last. A VertexColor animation blends its
/// target's colour; RemoveAnimations empties a group; Restart's offset starts part-way through.
#[test]
fn curves_and_colour_animations_follow_classicapi() {
    let script = extended();
    script
        .run(r#"
        local c = C_CurveUtil.CreateCurve()
        assert(c:Evaluate(3) == 0 and c:GetType() == Enum.LuaCurveType.Linear)
        c:AddPoint(0, 0); c:AddPoint(10, 1); c:AddPoint(5, 0.25)
        assert(c:GetPointCount() == 3 and c:GetPoint(2).x == 5)
        assert(c:Evaluate(-4) == 0 and c:Evaluate(99) == 1 and c:Evaluate(2.5) == 0.125)
        c:SetType(Enum.LuaCurveType.Step); assert(c:Evaluate(7) == 0.25)
        local k = C_CurveUtil.CreateCurve(); k:SetType(Enum.LuaCurveType.Cubic)
        for x = -1, 3 do k:AddPoint(x, x * 2) end
        assert(k:Evaluate(0.5) == 1 and k:Evaluate(-1) == 0 and k:Evaluate(9) == 4)
        assert(not pcall(k.SetType, k, 9))

        local f = CreateFrame('Frame'); tex = f:CreateTexture()
        group = f:CreateAnimationGroup()
        for i, pair in ipairs({ {0, 1}, {1, 0} }) do
          local a = group:CreateAnimation('VertexColor')
          a:SetOrder(i); a:SetDuration(1)
          a:SetStartColor(CreateColor(pair[1], 0, 0, 1)); a:SetEndColor(CreateColor(pair[2], 0, 0, 1))
          a:SetTarget(tex)
        end
        group:SetLooping('REPEAT')
        group:Restart(false, 1.5)
        local r = tex:GetVertexColor()
        assert(math.abs(r - 0.5) <= 1 / 255, 'halfway down the second order: ' .. r)
        group:RemoveAnimations()
        assert(not group:IsPlaying() and select('#', group:GetAnimations()) == 0)
        "#)
        .unwrap();
    assert!(script.errors().is_empty(), "{:?}", script.errors());
}

/// ClassicAPI's explicit loads (`item/Data.cpp`, `quest/Data.cpp`): a cached record answers inside
/// the call, an uncached one asks and answers when it lands, once per call made while it waited;
/// an item id of 0 or less answers false at once.
#[test]
fn explicit_item_and_quest_loads_answer_once_per_call() {
    use benilla_ui::script::{pfui_host, ItemTemplateView, QuestRecordView};
    let mut script = extended();
    script.set_item_template(
        6948,
        ItemTemplateView {
            name: "Hearthstone".into(),
            ..Default::default()
        },
    );
    script
        .run(r#"
        got = {}
        local f = CreateFrame('Frame')
        f:RegisterEvent('ITEM_DATA_LOAD_RESULT'); f:RegisterEvent('QUEST_DATA_LOAD_RESULT')
        f:SetScript('OnEvent', function() table.insert(got, event .. ' ' .. arg1 .. ' ' .. tostring(arg2)) end)
        C_Item.RequestLoadItemDataByID(6948)
        C_Item.RequestLoadItemDataByID(0)
        C_Item.RequestLoadItemDataByID('item:2589')
        C_Item.RequestLoadItemDataByID(2589)
        C_QuestLog.RequestLoadQuestByID(783)
        "#)
        .unwrap();
    pfui_host::item_result(script.lua(), 2589, true);
    script.land_quest_records(vec![(
        783,
        QuestRecordView {
            title: "A Threat Within".into(),
            ..Default::default()
        },
    )]);
    script.run("C_QuestLog.RequestLoadQuestByID(783)").unwrap();
    let got: String = script.eval("return table.concat(got, ';')").unwrap();
    assert_eq!(
        got,
        "ITEM_DATA_LOAD_RESULT 6948 true;ITEM_DATA_LOAD_RESULT 0 false;\
         ITEM_DATA_LOAD_RESULT 2589 true;ITEM_DATA_LOAD_RESULT 2589 true;\
         QUEST_DATA_LOAD_RESULT 783 true;QUEST_DATA_LOAD_RESULT 783 true"
    );
}

/// ClassicAPI's `boss1`..`boss5` are tokens (no unit until an encounter names one) and other
/// indices are not; `C_InstanceEncounter` keeps the creature IDs addons name bosses, a bad entry
/// changing nothing; `C_Item.DoesItemExistByID` takes any positive id (`unit/Boss.cpp`,
/// `combatlog/BossCreatures.cpp`, `item/Exists.cpp`).
#[test]
fn boss_tokens_and_the_boss_creature_list() {
    let script = extended();
    script
        .run(r#"
        assert(not UnitExists('boss1') and not UnitExists('boss5target') and UnitGUID('boss3') == nil)
        assert(not pcall(UnitExists, 'boss6') and not pcall(UnitExists, 'boss'))
        C_InstanceEncounter.AddBossCreatureID(11502)
        C_InstanceEncounter.AddBossCreatureID({12118, 11502, 11982})
        assert(not pcall(C_InstanceEncounter.AddBossCreatureID, {12057, 'x'}))
        C_InstanceEncounter.RemoveBossCreatureID(12118)
        local ids = C_InstanceEncounter.GetBossCreatureIDs()
        assert(#ids == 2 and ids[1] == 11502 and ids[2] == 11982)
        assert(C_Item.DoesItemExistByID(6948) and C_Item.DoesItemExistByID('item:6948:0:0:0'))
        assert(not C_Item.DoesItemExistByID(0) and not C_Item.DoesItemExistByID(nil))
        "#)
        .unwrap();
}

/// ClassicAPI 1.23's `<ModelScene>`: a frame of its own type that keeps its flags (the dressing
/// room's mount scene and its control frame load from it), and `C_MountJournal` by spell, a mount
/// being a spell with a SPELL_AURA_MOUNTED effect (`mount/Journal.cpp`).
#[test]
fn a_model_scene_loads_and_mounts_are_their_spells() {
    let script = extended();
    let xml = r#"<Ui>
        <ModelScene name="ProbeSceneTemplate" mixin="ProbeSceneMixin" virtual="true">
          <Scripts><OnLoad>self.loaded = true; self:SetAllowOverlappedModels(true)</OnLoad></Scripts>
        </ModelScene>
        <ModelScene name="ProbeScene" inherits="ProbeSceneTemplate">
          <Frames><Frame parentKey="ControlFrame"/></Frames>
        </ModelScene>
    </Ui>"#;
    script.run("ProbeSceneMixin = { Kind = 'probe' }").unwrap();
    let doc = benilla_ui::framexml::parse(xml).expect("parses");
    let report = benilla_ui::loader::load(&script, &doc, &|_| None);
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    script
        .run(r#"
        assert(ProbeScene and ProbeScene.loaded and ProbeScene.Kind == 'probe' and ProbeScene.ControlFrame)
        assert(ProbeScene:GetObjectType() == 'ModelScene' and ProbeScene:IsObjectType('ModelScene'))
        assert(ProbeScene:IsObjectType('Frame') and ProbeScene:GetAllowOverlappedModels() == true)
        assert(C_MountJournal.GetMountInfoExtraByID(458) == nil)
        assert(C_MountJournal.GetMountFromSpell(nil) == nil)
        "#)
        .unwrap();
    assert!(script.errors().is_empty(), "{:?}", script.errors());
}

/// ClassicAPI's `UnitDistanceSquared`: the raw squared distance from the player and whether a
/// position was read; the player is (0, true), a unit with none (0, false) (`unit/Range.cpp`).
#[test]
fn unit_distance_squared_reads_the_units_reach() {
    use benilla_ui::script::{UnitGuids, UnitReach};
    let mut script = extended();
    script.set_unit_guids(&UnitGuids {
        player: 3,
        target: 5,
        mouseover: 6,
        ..Default::default()
    });
    script.set_unit_reach(
        [(
            5,
            UnitReach {
                dist_sq: 900.0,
                inspectable: false,
            },
        )]
        .into_iter()
        .collect(),
    );
    script
        .run(
            r#"
        local d, ok = UnitDistanceSquared('target'); assert(d == 900 and ok == true)
        d, ok = UnitDistanceSquared('player'); assert(d == 0 and ok == true)
        d, ok = UnitDistanceSquared('mouseover'); assert(d == 0 and ok == false)
        d, ok = UnitDistanceSquared('party1'); assert(d == 0 and ok == false)
        assert(not pcall(UnitDistanceSquared))
        "#,
        )
        .unwrap();
}

/// ClassicAPI's `C_Item.GetStackCount`: the stack in one slot, here by the item's GUID as
/// `GetItemGUID` writes it; an unknown GUID, an empty location or another value is 0
/// (`item/StackCount.cpp`).
#[test]
fn stack_count_reads_the_items_own_stack() {
    use benilla_ui::script::pfui_host::items::{self, Instance};
    let script = extended();
    items::set_instances(
        script.lua(),
        [(
            (0, 1),
            Instance {
                guid: 0x4000_0000_0000_0042,
                item_id: 4306,
                count: 17,
                uses: 0,
                name: None,
            },
        )],
    );
    script
        .run(
            r#"
        assert(C_Item.GetStackCount('0x4000000000000042') == 17)
        assert(C_Item.GetStackCount('0x4000000000000043') == 0)
        assert(C_Item.GetStackCount({bagID = 3, slotIndex = 9}) == 0)
        assert(C_Item.GetStackCount(nil) == 0 and C_Item.GetStackCount(4306) == 0)
        "#,
        )
        .unwrap();
}

/// ClassicAPI's item-location calls (`item/Location.cpp` and its callers): a bag slot, an
/// equipment slot and an item GUID each name the item; an empty place is nil or false, a value
/// that is no location raises the usage error, and `LockItem`/`UnlockItem` set the engine's lock
/// with its `ITEM_LOCK_CHANGED`. These are what `ItemMixin` (`Item:CreateFromBagAndSlot`) calls.
#[test]
fn item_locations_name_a_bag_slot_an_equipment_slot_and_a_guid() {
    use benilla_ui::script::{ContainerSlot, ContainerState, InvSlotView, ItemTemplateView};
    use std::collections::HashMap;
    let mut script = extended();
    script.set_item_template(
        6948,
        ItemTemplateView {
            name: "Hearthstone".into(),
            quality: 1,
            stackable: 1,
            icon: Some("Interface\\Icons\\INV_Misc_Rune_01".into()),
            ..Default::default()
        },
    );
    script.set_container(
        0,
        Some(ContainerState {
            num_slots: 16,
            slots: HashMap::from([(
                1,
                ContainerSlot {
                    item_id: 6948,
                    guid: 0x4000_0000_0000_0042,
                    link: Some("|cffffffff|Hitem:6948:0:0:0|h[Hearthstone]|h|r".into()),
                    count: 1,
                    ..Default::default()
                },
            )]),
            ..Default::default()
        }),
    );
    let mut worn: benilla_ui::script::InventorySlots = Default::default();
    worn[1] = Some(InvSlotView {
        item_id: 10,
        guid: 0x4000_0000_0000_0051,
        name: Some("Cap".into()),
        quality: 1,
        ..Default::default()
    });
    script.set_inventory_slots(worn);
    script
        .run(
            r#"
        local bag, head = {bagID = 0, slotIndex = 1}, {equipmentSlotIndex = 1}
        assert(C_Item.DoesItemExist(bag) and C_Item.DoesItemExist(head))
        assert(C_Item.DoesItemExist({bagID = 0, slotIndex = 2}) == false)
        assert(C_Item.DoesItemExist(nil) == false and C_Item.DoesItemExist(6948) == false)
        assert(C_Item.GetItemGUID(bag) == "0x4000000000000042")
        assert(C_Item.GetItemGUID(head) == "0x4000000000000051")
        assert(C_Item.GetItemGUID({bagID = 0, slotIndex = 2}) == nil)
        assert(C_Item.GetItemLink(bag) == GetContainerItemLink(0, 1))
        assert(C_Item.GetItemLink(head) == GetInventoryItemLink("player", 1))
        assert(C_Item.GetItemLink("0x4000000000000042") == GetContainerItemLink(0, 1))
        assert(C_Item.GetItemIcon(bag) == "Interface\\Icons\\INV_Misc_Rune_01")
        assert(GetItemIcon(6948) == "Interface\\Icons\\INV_Misc_Rune_01")
        assert(C_Item.GetItemMaxStackSize(bag) == 1)
        assert(C_Item.IsItemDataCached(bag) == true)
        assert(C_Item.GetItemIDForItemInfo("item:6948") == 6948)
        assert(select('#', C_Item.GetItemIDForItemInfo("Hearthstone")) == 0)
        local at = C_Item.GetItemLocation("0x4000000000000042")
        assert(at.bagID == 0 and at.slotIndex == 1 and at.equipmentSlotIndex == nil)
        at = C_Item.GetItemLocation("0x4000000000000051")
        assert(at.equipmentSlotIndex == 1 and at.bagID == nil)
        assert(C_Item.GetItemLocation("0x4000000000000099") == nil)
        for _, call in ipairs({"GetItemLink", "GetItemGUID", "GetItemIcon", "GetItemMaxStackSize", "LockItem", "UnlockItem"}) do
            local ok, err = pcall(C_Item[call], 5)
            assert(not ok and string.find(err, "Usage: C_Item." .. call .. "(itemLocation)", 1, true), call)
        end
        assert(not pcall(C_Item.IsItemDataCached, {bagID = 0, slotIndex = 2}))
        assert(not pcall(GetItemIcon, "item:6948"))
        local changes = 0
        local f = CreateFrame("Frame")
        f:RegisterEvent("ITEM_LOCK_CHANGED")
        f:SetScript("OnEvent", function() changes = changes + 1 end)
        assert(C_Item.IsLocked(bag) == false and C_Item.IsLocked(7) == false)
        C_Item.LockItem(bag)
        assert(C_Item.IsLocked(bag) == true and select(3, GetContainerItemInfo(0, 1)) and changes == 1)
        C_Item.UnlockItem("0x4000000000000042")
        assert(C_Item.IsLocked(bag) == false and changes == 2)
        C_Item.LockItem({bagID = 0, slotIndex = 2})
        assert(changes == 2, "an empty place locks nothing")
        "#,
        )
        .unwrap();
}

/// ClassicAPI's GUID registrations (`frame/GuidEvent.cpp`): a frame takes a unit event for a GUID
/// it registered with `arg1` the GUID, once per event and frame of the clock whether or not a
/// token names the unit, apart from its own event registrations; `UnregisterAllEvents` drops
/// them. `UNIT_VISIBILITY_CHANGED` fires as a unit enters or leaves view, after a seeding feed
/// (`unit/VisibilityEvent.cpp`).
#[test]
fn guid_registrations_take_their_units_events() {
    use benilla_ui::script::pfui_host::{
        auras::{self, Aura},
        units::{self, RawUnit},
    };
    use benilla_ui::script::{AuraState, UnitGuids};
    let mut script = extended();
    script.set_unit_guids(&UnitGuids {
        player: 3,
        target: 5,
        ..Default::default()
    });
    let raw = |guid: u64| {
        (
            guid,
            RawUnit {
                health: Some(1),
                form: 0,
                mount_display: 0,
                auras: Vec::new(),
                applications: Vec::new(),
            },
        )
    };
    let aura = |spell_id: u32| Aura {
        crowd_control: false,
        self_only: false,
        applies_aura: false,
        immunities: Vec::new(),
        slot: 0,
        state: AuraState {
            spell_id,
            helpful: true,
            name: Some("Mark".into()),
            ..Default::default()
        },
        source_guid: None,
    };
    units::set_raw_units(script.lua(), [raw(5)]);
    script
        .run(r#"
        seen = {}
        f = CreateFrame('Frame', 'GuidProbe')
        f:SetScript('OnEvent', function() table.insert(seen, event .. ' ' .. arg1 .. ' ' .. tostring(arg2)) end)
        assert(f:RegisterGUIDEvent('UNIT_AURA', '0x0000000000000007', 'junk', '0x0000000000000007') == true)
        assert(f:RegisterGUIDEvent('UNIT_AURA', '0x0000000000000007') == false, 'unchanged')
        assert(f:RegisterGUIDEvent('UNIT_VISIBILITY_CHANGED', '0x0000000000000007', '0x0000000000000005'))
        local ok, guid = f:IsGUIDEventRegistered('UNIT_AURA')
        assert(ok == true and guid == '0x0000000000000007')
        assert(select(2, f:IsGUIDEventRegistered('UNIT_HEALTH')) == nil)
        assert(not f:IsEventRegistered('UNIT_AURA'), 'apart from the event registrations')
        assert(not pcall(f.RegisterGUIDEvent, f, 'PLAYER_LOGIN', '0x0000000000000007'))
        assert(not pcall(f.RegisterGUIDEvent, f, 'UNIT_AURA', 'junk'))
        "#)
        .unwrap();
    // No token names 7: the feed's own signal reaches the frame, once.
    auras::set_auras(script.lua(), 7, Some(vec![aura(201)]));
    units::set_raw_units(script.lua(), [raw(5), raw(7)]);
    script.set_now(1.0);
    units::set_raw_units(script.lua(), [raw(7)]);
    let seen: String = script.eval("return table.concat(seen, ';')").unwrap();
    assert_eq!(
        seen,
        "UNIT_AURA 0x0000000000000007 nil;\
         UNIT_VISIBILITY_CHANGED 0x0000000000000007 true;\
         UNIT_VISIBILITY_CHANGED 0x0000000000000005 false"
    );
    script
        .run("f:UnregisterAllEvents(); seen = {}; assert(not f:IsGUIDEventRegistered('UNIT_AURA'))")
        .unwrap();
    script.set_now(2.0);
    auras::set_auras(script.lua(), 7, Some(vec![aura(202)]));
    let seen: String = script.eval("return table.concat(seen, ';')").unwrap();
    assert_eq!(seen, "");
    assert!(script.errors().is_empty(), "{:?}", script.errors());
}

/// ClassicAPI's `SetPassThroughButtons`: a press of a passed button skips the frame and reaches the
/// mouse frame under it, or the world with none; its other buttons stay its own, and no argument
/// clears the set (`frame/MouseInput.cpp`).
#[test]
fn a_passed_button_reaches_the_frame_under_it() {
    use benilla_ui::script::PointerTarget;
    let mut s = extended();
    s.set_screen_size(800.0, 600.0);
    s.run(
        r#"
        UIParent = CreateFrame("Frame", "UIParent"); UIParent:SetAllPoints()
        presses = {}
        local function frame(name, size)
            local f = CreateFrame("Button", name, UIParent)
            f:SetPoint("BOTTOMLEFT", 100, 100); f:SetWidth(size); f:SetHeight(size)
            f:EnableMouse(true)
            f:SetScript("OnMouseDown", function() table.insert(presses, this:GetName() .. ":" .. arg1) end)
            return f
        end
        Under = frame("Under", 80)
        Over = frame("Over", 40); Over:SetFrameLevel(Under:GetFrameLevel() + 5)
        Over:SetPassThroughButtons("LeftButton", "Nonsense")
        assert(not pcall(Over.SetPassThroughButtons, Over, 1))
        "#,
    )
    .unwrap();
    s.resolve();
    s.mouse_move(120.0, 120.0);
    for button in ["LeftButton", "RightButton"] {
        s.mouse_button(120.0, 120.0, button, true);
        s.mouse_button(120.0, 120.0, button, false);
    }
    s.run("Under:Hide()").unwrap();
    s.resolve();
    s.mouse_move(121.0, 121.0);
    let target = s.mouse_button_to(121.0, 121.0, "LeftButton", true);
    s.mouse_button(121.0, 121.0, "LeftButton", false);
    assert_eq!(
        target,
        PointerTarget::World,
        "nothing under it: the world takes it"
    );
    s.run("Over:SetPassThroughButtons()").unwrap();
    s.mouse_button(121.0, 121.0, "LeftButton", true);
    s.mouse_button(121.0, 121.0, "LeftButton", false);
    let presses: String = s.eval("return table.concat(presses, ' ')").unwrap();
    assert_eq!(presses, "Under:LeftButton Over:RightButton Over:LeftButton");
    assert!(s.take_errors().is_empty());
}

/// ClassicAPI's `C_GossipInfo` reads the open window's wire rows and selects by option or quest id
/// through the stock selectors, at the row's place; a closed window lists nothing and sends
/// nothing (`gossip/Info.cpp`).
#[test]
fn gossip_info_lists_the_window_and_selects_by_id() {
    use benilla_ui::script::live::LiveAsk;
    use benilla_ui::script::pfui_host::gossip_info::{GossipInfo, GossipInfoQuery};
    use benilla_ui::script::{CallHost, CallTail, GossipCall, ScriptCall};
    struct Npc(Vec<GossipCall>, Option<GossipInfo>);
    impl CallHost for Npc {
        fn apply(&mut self, _: &mut UiScript, call: ScriptCall) -> Option<CallTail> {
            if let ScriptCall::Gossip(call) = call {
                self.0.push(call);
            }
            None
        }
        fn read_live(&mut self, ask: &mut LiveAsk<'_>) -> bool {
            let info = self.1.clone();
            ask.answer(|_: &GossipInfoQuery| Some(info))
        }
    }
    let mut script = extended();
    let mut closed = Npc(Vec::new(), None);
    script.hosted(&mut closed, |s| {
        s.run(r#"
            assert(next(C_GossipInfo.GetOptions()) == nil and C_GossipInfo.GetNumOptions() == 0)
            assert(GetNumGossipAvailableQuests() == 0 and next(C_GossipInfo.GetActiveQuests()) == nil)
            C_GossipInfo.SelectOption(3); C_GossipInfo.SelectAvailableQuest(100)
        "#).unwrap()
    });
    assert!(closed.0.is_empty(), "a closed window sends nothing");
    let mut open = Npc(
        Vec::new(),
        Some(GossipInfo {
            options: vec![(3, 1, "Train me".into()), (7, 0, "Bye".into())],
            quests: vec![
                (100, 2, 10, "Find it".into()),
                (200, 4, 12, "Bring it".into()),
                (300, 3, 11, "Still out".into()),
            ],
        }),
    );
    script.hosted(&mut open, |s| {
        s.run(r#"
            local o = C_GossipInfo.GetOptions()
            assert(#o == 2 and o[2].gossipOptionID == 7 and o[2].name == "Bye" and o[2].orderIndex == 2)
            assert(o[1].icon == 1 and o[1].status == 0 and o[1].flags == 0)
            assert(GetNumGossipOptions() == 2 and C_GossipInfo.GetNumAvailableQuests() == 1)
            local a, act = C_GossipInfo.GetAvailableQuests(), C_GossipInfo.GetActiveQuests()
            assert(#a == 1 and a[1].questID == 100 and a[1].questLevel == 10 and a[1].isComplete == nil)
            assert(#act == 2 and act[1].questID == 200 and act[1].isComplete == true)
            assert(act[2].questID == 300 and act[2].isComplete == false and GetNumGossipActiveQuests() == 2)
            C_GossipInfo.SelectOption(7)
            C_GossipInfo.SelectOptionByIndex(1)
            C_GossipInfo.SelectOption(99)
            C_GossipInfo.SelectActiveQuest(300)
            C_GossipInfo.SelectAvailableQuest(100)
            assert(not pcall(C_GossipInfo.SelectOption))
        "#).unwrap()
    });
    assert_eq!(
        open.0,
        vec![
            GossipCall::Option {
                index: 2,
                code: None
            },
            GossipCall::Option {
                index: 1,
                code: None
            },
            GossipCall::Quest {
                active: true,
                index: 2
            },
            GossipCall::Quest {
                active: false,
                index: 1
            },
        ]
    );
}

/// pfUI's mirror bars read the value at the call (`mirrortimers.lua:11`): a draining breath bar
/// counts down from its start by its scale, a paused one holds, a stop empties the slot.
#[test]
fn mirror_timer_progress_runs_from_the_start_by_its_scale() {
    use benilla_ui::script::pfui_host::mirror_timers::{self, MirrorTimerStart};
    let mut script = extended();
    script
        .run(r#"
            assert(GetMirrorTimerProgress('BREATH')==0)
            assert(GetMirrorTimerProgress('nothing')==nil)
            local name,value,max,scale,paused,label,spell=GetMirrorTimerInfo(2)
            assert(name=='UNKNOWN' and value==0 and max==0 and scale==0 and paused==0 and label=='' and spell==0)
            assert(GetMirrorTimerInfo(4)==nil)
            assert(not pcall(GetMirrorTimerProgress))
        "#)
        .unwrap();
    script.set_now(10.0);
    let breath = MirrorTimerStart {
        kind: 1,
        value: 60_000,
        max: 60_000,
        scale: -1,
        paused: false,
        spell_id: 0,
        label: "Breath",
    };
    mirror_timers::start(script.lua(), breath);
    script.set_now(12.5);
    script
        .run(r#"
            assert(GetMirrorTimerProgress('breath')==57500)
            local name,value,max,scale,paused,label=GetMirrorTimerInfo('2')
            assert(name=='BREATH' and value==60000 and max==60000 and scale==-1 and paused==0 and label=='Breath')
        "#)
        .unwrap();
    mirror_timers::start(
        script.lua(),
        MirrorTimerStart {
            kind: 1,
            value: 30_000,
            max: 60_000,
            scale: 10,
            paused: true,
            spell_id: 0,
            label: "Breath",
        },
    );
    script.set_now(20.0);
    script
        .run("assert(GetMirrorTimerProgress('BREATH')==30000)")
        .unwrap();
    mirror_timers::stop(script.lua(), 1);
    mirror_timers::stop(script.lua(), 7);
    script
        .run("assert(GetMirrorTimerProgress('BREATH')==0 and GetMirrorTimerInfo(2)=='UNKNOWN')")
        .unwrap();
}

#[test]
fn totem_queries_distinguish_tools_from_active_summons_and_target_real_guids() {
    use benilla_ui::script::{
        pfui_host::{
            items::{self, Instance},
            totems::{self, Totem},
        },
        ScriptCall, SelectionRequest,
    };
    let mut script = extended();
    totems::set(script.lua(), std::array::from_fn(|_| None), [5176, 0, 0, 0]);
    script.run("local have,name,start,duration,icon,rate,id=GetTotemInfo(FIRE_TOTEM_SLOT); assert(not have and name=='' and start==0 and duration==0 and icon==nil and rate==1 and id==0)").unwrap();
    items::set_instances(
        script.lua(),
        [(
            (0, 1),
            Instance {
                guid: 100,
                item_id: 5176,
                count: 1,
                uses: 1,
                name: None,
            },
        )],
    );
    totems::set(script.lua(), std::array::from_fn(|_| None), [5176, 0, 0, 0]);
    script
        .run("assert(GetTotemInfo(1)==true); TargetTotem(1)")
        .unwrap();
    assert!(script.take_script_calls().is_empty());
    let active = Totem {
        spell_id: 3599,
        guid: Some(77),
        name: "Searing Totem".into(),
        icon: Some("Icon".into()),
        start: 2.,
        duration: 30.,
    };
    totems::set(
        script.lua(),
        [Some(active), None, None, None],
        [5176, 0, 0, 0],
    );
    script.run("local have,name,start,duration,icon,rate,id=GetTotemInfo(1); assert(have and name=='Searing Totem' and start==2 and duration==30 and icon=='Icon' and id==3599); TargetTotem(1)").unwrap();
    assert_eq!(
        script.take_script_calls(),
        vec![ScriptCall::Select {
            request: SelectionRequest::PfuiGuid(77),
            may_cast: true
        }]
    );
    totems::set(script.lua(), std::array::from_fn(|_| None), [5176, 0, 0, 0]);
    script
        .run("local have,name=GetTotemInfo(1); assert(have and name=='')")
        .unwrap();
}

/// The modern list puts the player's own auras first, so its index is not the stock one: an aura
/// the player cast in a later slot leads it. `SetUnitAura` turns the modern index into the stock
/// walk's before it calls the stock setter, as ClassicAPI does (`FindNthSlot`, then
/// `EngineIndexForSlot`), so the tooltip names the aura the icon shows.
#[test]
fn aura_tooltip_index_follows_the_modern_order() {
    use benilla_ui::script::{
        pfui_host::auras::{self, Aura},
        AuraState, SpellTooltipView, UnitGuids,
    };
    let mut script = extended();
    script.set_unit_guids(&UnitGuids {
        player: 3,
        target: 5,
        ..Default::default()
    });
    let buff = |slot: u8, spell_id: u32, name: &str, source_guid: Option<u64>| Aura {
        crowd_control: false,
        self_only: false,
        applies_aura: false,
        immunities: Vec::new(),
        slot,
        state: AuraState {
            spell_id,
            slot,
            name: Some(name.into()),
            helpful: true,
            ..Default::default()
        },
        source_guid,
    };
    for (id, name) in [(201, "Mark"), (202, "Rejuvenation")] {
        script.set_spell_tooltip(
            id,
            SpellTooltipView {
                name: name.into(),
                ..Default::default()
            },
        );
    }
    // Slot 0 from someone unseen, slot 1 the player's own cast.
    auras::set_auras(
        script.lua(),
        5,
        Some(vec![
            buff(0, 201, "Mark", None),
            buff(1, 202, "Rejuvenation", Some(3)),
        ]),
    );
    script
        .run(
            r#"
        local tip=CreateFrame('GameTooltip','ProbeOrderTip',nil,'TestTooltipTemplate')
        local function at(i)
          tip:SetOwner(WorldFrame or CreateFrame('Frame'),'ANCHOR_NONE')
          tip:SetUnitAura('target',i,'HELPFUL')
          return ProbeOrderTipTextLeft1:GetText()
        end
        for i=1,2 do
          local icon=C_UnitAuras.GetAuraDataByIndex('target',i,'HELPFUL').name
          assert(at(i)==icon, i..': icon '..icon..', tooltip '..tostring(at(i)))
        end
        assert(at(1)=='Rejuvenation', 'own first')
    "#,
        )
        .unwrap();
    assert!(script.errors().is_empty(), "{:?}", script.errors());
}

/// ClassicAPI's aura instance IDs (`aura/Instance.cpp`): a slot keeps its ID while it holds the
/// same spell, new stacks and all, and a new spell there gets a fresh one. The by-ID getters and
/// tooltip setters find the aura by it, the filter decides, and a Buff/Debuff form forces its
/// range (`aura/Api.cpp`, `unit/Tooltip.cpp`).
#[test]
fn aura_instance_ids_follow_the_slot_and_name_the_aura() {
    use benilla_ui::script::{
        pfui_host::auras::{self, Aura},
        AuraState, SpellTooltipView, UnitGuids,
    };
    let mut script = extended();
    script.set_unit_guids(&UnitGuids {
        player: 3,
        target: 5,
        ..Default::default()
    });
    let aura = |slot: u8, spell_id: u32, name: &str, helpful: bool, count: u8| Aura {
        crowd_control: false,
        self_only: false,
        applies_aura: false,
        immunities: Vec::new(),
        slot,
        state: AuraState {
            spell_id,
            slot,
            count,
            name: Some(name.into()),
            helpful,
            ..Default::default()
        },
        source_guid: None,
    };
    for (id, name) in [(201, "Mark"), (301, "Sunder"), (202, "Thorns")] {
        script.set_spell_tooltip(
            id,
            SpellTooltipView {
                name: name.into(),
                ..Default::default()
            },
        );
    }
    auras::set_auras(
        script.lua(),
        5,
        Some(vec![
            aura(0, 201, "Mark", true, 1),
            aura(40, 301, "Sunder", false, 1),
        ]),
    );
    script
        .run(r#"
        local mark = C_UnitAuras.GetAuraDataByIndex('target', 1, 'HELPFUL')
        local sunder = C_UnitAuras.GetAuraDataByIndex('target', 1, 'HARMFUL')
        assert(mark.auraInstanceID and sunder.auraInstanceID and mark.auraInstanceID ~= sunder.auraInstanceID)
        assert(select(16, C_UnitAuras.UnitAura('target', 1, 'HELPFUL')) == mark.auraInstanceID)
        MARK, SUNDER = mark.auraInstanceID, sunder.auraInstanceID
        assert(C_UnitAuras.GetAuraDataByAuraInstanceID('target', SUNDER).name == 'Sunder')
        local buf = { stale = true }
        assert(C_UnitAuras.GetAuraDataByAuraInstanceID('target', MARK, buf) == buf and buf.stale == nil and buf.name == 'Mark')
        assert(C_UnitAuras.GetAuraDataByAuraInstanceID('target', 9999) == nil)
        assert(C_UnitAuras.IsAuraFilteredOutByInstanceID('target', MARK, 'HARMFUL'))
        assert(not C_UnitAuras.IsAuraFilteredOutByInstanceID('target', MARK, ''))
        assert(C_UnitAuras.IsAuraFilteredOutByInstanceID('target', 9999, ''))
        tip = CreateFrame('GameTooltip', 'ProbeIdTip', nil, 'TestTooltipTemplate')
        function show(method, id, filter)
          tip:SetOwner(CreateFrame('Frame'), 'ANCHOR_NONE'); tip:ClearLines()
          tip[method](tip, 'target', id, filter)
          return ProbeIdTipTextLeft1:GetText()
        end
        assert(show('SetUnitAuraByAuraInstanceID', SUNDER) == 'Sunder')
        assert(show('SetUnitBuffByAuraInstanceID', MARK, 'HARMFUL') == 'Mark', 'Buff forces HELPFUL')
        assert(show('SetUnitDebuffByAuraInstanceID', MARK) == nil, 'Debuff rejects a buff')
        assert(show('SetUnitAuraByAuraInstanceID', MARK, 'HARMFUL') == nil, 'the filter rejects it')
        assert(not pcall(tip.SetUnitAuraByAuraInstanceID, tip, 'target'))
        "#)
        .unwrap();
    // Mark gains a stack: same instance. Thorns replaces Sunder's slot: a new one.
    auras::set_auras(
        script.lua(),
        5,
        Some(vec![
            aura(0, 201, "Mark", true, 2),
            aura(40, 202, "Thorns", true, 1),
        ]),
    );
    script
        .run(
            r#"
        assert(C_UnitAuras.GetAuraDataBySlot('target', 0).auraInstanceID == MARK)
        local thorns = C_UnitAuras.GetAuraDataBySlot('target', 40).auraInstanceID
        assert(thorns ~= SUNDER and thorns ~= MARK)
        assert(C_UnitAuras.GetAuraDataByAuraInstanceID('target', SUNDER) == nil)
        "#,
        )
        .unwrap();
    auras::set_auras(script.lua(), 5, None);
    auras::set_auras(script.lua(), 5, Some(vec![aura(0, 201, "Mark", true, 2)]));
    script
        .run("assert(C_UnitAuras.GetAuraDataBySlot('target', 0).auraInstanceID ~= MARK, 'seen again: new IDs')")
        .unwrap();
    assert!(script.errors().is_empty(), "{:?}", script.errors());
}

/// ClassicAPI's `UNIT_AURA_DELTA(unit, updateInfo)` (`aura/Delta.cpp`): a unit's first list is a
/// full update with its IDs added; a restack is an update of the same ID; a replaced or lost aura
/// is removed; each per token naming the unit and to the frames that registered its GUID.
#[test]
fn unit_aura_delta_reports_instance_changes() {
    use benilla_ui::script::{
        pfui_host::auras::{self, Aura},
        AuraState, UnitGuids,
    };
    let mut script = extended();
    script.set_unit_guids(&UnitGuids {
        player: 3,
        target: 5,
        ..Default::default()
    });
    let aura = |slot: u8, spell_id: u32, count: u8| Aura {
        crowd_control: false,
        self_only: false,
        applies_aura: false,
        immunities: Vec::new(),
        slot,
        state: AuraState {
            spell_id,
            slot,
            count,
            helpful: true,
            ..Default::default()
        },
        source_guid: None,
    };
    script
        .run(r#"
        log = {}
        local function ids(t) return t and table.concat(t, ',') or '-' end
        local function note() table.insert(log, arg1 .. ' ' .. tostring(arg2.isFullUpdate) .. ' +' .. ids(arg2.addedAuraInstanceIDs) .. ' ~' .. ids(arg2.updatedAuraInstanceIDs) .. ' -' .. ids(arg2.removedAuraInstanceIDs)) end
        local token = CreateFrame('Frame'); token:RegisterUnitEvent('UNIT_AURA_DELTA', 'target'); token:SetScript('OnEvent', note)
        local byguid = CreateFrame('Frame'); byguid:RegisterGUIDEvent('UNIT_AURA_DELTA', '0x0000000000000005'); byguid:SetScript('OnEvent', note)
        "#)
        .unwrap();
    auras::set_auras(script.lua(), 5, Some(vec![aura(0, 201, 1)]));
    script.set_now(1.0);
    auras::set_auras(script.lua(), 5, Some(vec![aura(0, 201, 2)]));
    script.set_now(2.0);
    auras::set_auras(script.lua(), 5, Some(vec![aura(0, 202, 1)]));
    let log: String = script.eval("return table.concat(log, ';')").unwrap();
    let ids: Vec<&str> = log.split(';').collect();
    assert_eq!(ids.len(), 6, "{log}");
    assert!(
        ids[0].starts_with("target true +") && ids[0].ends_with(" ~- --"),
        "{log}"
    );
    assert!(ids[1].starts_with("0x0000000000000005 true +"), "{log}");
    let first = ids[0]
        .trim_start_matches("target true +")
        .split(' ')
        .next()
        .unwrap()
        .to_string();
    assert_eq!(ids[2], format!("target false +- ~{first} --"));
    assert!(
        ids[4].starts_with("target false +") && ids[4].ends_with(&format!(" ~- -{first}")),
        "{log}"
    );
    assert!(script.errors().is_empty(), "{:?}", script.errors());
}

#[test]
fn aura_tooltips_use_the_modern_index_space_and_existing_spell_renderer() {
    use benilla_ui::script::{
        pfui_host::auras::{self, Aura},
        AuraState, SpellTooltipView, UnitGuids,
    };
    let mut script = extended();
    script.set_unit_guids(&UnitGuids {
        target: 5,
        ..Default::default()
    });
    script.set_spell_tooltip(
        123,
        SpellTooltipView {
            name: "Effect".into(),
            ..Default::default()
        },
    );
    auras::set_auras(
        script.lua(),
        5,
        Some(vec![Aura {
            crowd_control: false,
            self_only: false,
            applies_aura: false,
            immunities: Vec::new(),
            slot: 7,
            state: AuraState {
                spell_id: 123,
                name: Some("Effect".into()),
                helpful: false,
                ..Default::default()
            },
            source_guid: None,
        }]),
    );
    script
        .run(
            r#"
        local tip=CreateFrame('GameTooltip','ProbeTooltip',nil,'TestTooltipTemplate')
        local function shown()
          local lines={}
          for i=1,tip:NumLines() do lines[i]=getglobal('ProbeTooltipTextLeft'..i):GetText() end
          return table.concat(lines,'|')
        end
        -- The modern call draws what the stock aura tooltips draw for the filter's polarity.
        tip:SetOwner(WorldFrame or CreateFrame('Frame'),'ANCHOR_NONE')
        tip:SetUnitDebuff('target',1); local debuff=shown()
        tip:SetOwner(WorldFrame or CreateFrame('Frame'),'ANCHOR_NONE')
        tip:SetUnitAura('target',1,'HARMFUL'); assert(shown()==debuff)
        tip:SetOwner(WorldFrame or CreateFrame('Frame'),'ANCHOR_NONE')
        tip:SetUnitBuff('target',1); local buff=shown()
        tip:SetOwner(WorldFrame or CreateFrame('Frame'),'ANCHOR_NONE')
        tip:SetUnitAura('target',1,'HELPFUL'); assert(shown()==buff)
        tip:SetOwner(WorldFrame or CreateFrame('Frame'),'ANCHOR_NONE')
        tip:SetSpellByID(123); assert(ProbeTooltipTextLeft1:GetText()=='Effect')
    "#,
        )
        .unwrap();
    assert!(script.errors().is_empty(), "{:?}", script.errors());
}

#[test]
fn merchant_junk_sales_are_paced_and_resolve_moved_instance_guids() {
    use benilla_ui::script::{
        pfui_host::items::{self, Instance},
        ContainerSlot, ContainerState, ItemTemplateView, MerchantItem, MerchantState,
    };
    let mut script = extended();
    // The sales are calls; this VM has no host, so it keeps them.
    script.keep_calls();
    let sells = |script: &mut UiScript| -> Vec<u64> {
        script
            .take_script_calls()
            .into_iter()
            .filter_map(|call| match call {
                benilla_ui::script::ScriptCall::Merchant(
                    benilla_ui::script::MerchantCall::Sell { item_guid, .. },
                ) => Some(item_guid),
                _ => None,
            })
            .collect()
    };
    script.set_item_template(
        38,
        ItemTemplateView {
            quality: 0,
            ..Default::default()
        },
    );
    script.set_item_template(
        39,
        ItemTemplateView {
            quality: 1,
            ..Default::default()
        },
    );
    let bag = ContainerState {
        num_slots: 4,
        slots: (1..=3)
            .map(|slot| {
                (
                    slot,
                    ContainerSlot {
                        item_id: if slot == 3 { 39 } else { 38 },
                        count: 1,
                        ..Default::default()
                    },
                )
            })
            .collect(),
        ..Default::default()
    };
    script.set_container(0, Some(bag));
    let item = |guid, id| Instance {
        guid,
        item_id: id,
        count: 1,
        uses: 1,
        name: None,
    };
    items::set_instances(
        script.lua(),
        [
            ((0, 1), item(101, 38)),
            ((0, 2), item(102, 38)),
            ((0, 3), item(103, 39)),
        ],
    );
    script.run("assert(C_MerchantFrame.GetNumJunkItems()==0 and C_MerchantFrame.GetBuybackItemID(1)==nil); C_MerchantFrame.SellAllJunkItems()").unwrap();
    script.set_merchant(Some(MerchantState {
        buyback: vec![MerchantItem {
            item_id: 40,
            ..Default::default()
        }],
        ..Default::default()
    }));
    script.run("assert(C_MerchantFrame.GetNumJunkItems()==2 and C_MerchantFrame.GetBuybackItemID(1)==40); C_MerchantFrame.SellAllJunkItems()").unwrap();
    assert!(sells(&mut script).is_empty());
    items::set_instances(
        script.lua(),
        [
            ((0, 1), item(102, 38)),
            ((0, 2), item(101, 38)),
            ((0, 3), item(103, 39)),
        ],
    );
    // The first junk instance, 101, sold where it moved to.
    script.tick(0.1);
    assert_eq!(sells(&mut script), vec![101]);
    script.fire_event("MERCHANT_CLOSED", vec![]);
    script.tick(0.1);
    assert!(sells(&mut script).is_empty());
    // A new pass walks the bag as it stands: 102 at slot 1, then 101 at slot 2.
    script.run("C_MerchantFrame.SellAllJunkItems()").unwrap();
    script.tick(0.1);
    assert_eq!(sells(&mut script), vec![102]);
    script.tick(0.1);
    assert_eq!(sells(&mut script), vec![101]);
    assert!(script.errors().is_empty(), "{:?}", script.errors());
}

#[test]
fn merchant_junk_reads_the_vendor_live_with_the_host_lent() {
    use benilla_ui::script::{
        live::LiveAsk,
        pfui_host::items::{self, Instance},
        BuybackRowQuery, CallHost, CallTail, ContainerSlot, ContainerState, MerchantCall,
        MerchantHead, MerchantHeadQuery, MerchantItem, ScriptCall,
    };
    /// The client in play: the vendor open in the host only, the VM's copy never set.
    #[derive(Default)]
    struct Vendor {
        sold: Vec<u64>,
    }
    impl CallHost for Vendor {
        fn apply(&mut self, _: &mut UiScript, call: ScriptCall) -> Option<CallTail> {
            if let ScriptCall::Merchant(MerchantCall::Sell { item_guid, .. }) = call {
                self.sold.push(item_guid);
            }
            None
        }
        fn read_live(&mut self, ask: &mut LiveAsk<'_>) -> bool {
            ask.answer(|_: &MerchantHeadQuery| {
                Some(MerchantHead {
                    open: true,
                    buyback: 1,
                    ..Default::default()
                })
            }) || ask.answer(|q: &BuybackRowQuery| {
                Some((q.0 == 0).then(|| MerchantItem {
                    item_id: 40,
                    ..Default::default()
                }))
            })
        }
    }
    let mut script = extended();
    // No item template seated: the grey is known by the slot's own quality.
    let bag = ContainerState {
        num_slots: 4,
        slots: (1..=2)
            .map(|slot| {
                (
                    slot,
                    ContainerSlot {
                        item_id: 37 + slot,
                        count: 1,
                        quality: Some(slot - 1),
                        ..Default::default()
                    },
                )
            })
            .collect(),
        ..Default::default()
    };
    script.set_container(0, Some(bag));
    let item = |guid, id| Instance {
        guid,
        item_id: id,
        count: 1,
        uses: 1,
        name: None,
    };
    items::set_instances(
        script.lua(),
        [((0, 1), item(101, 38)), ((0, 2), item(102, 39))],
    );
    script
        .run("assert(C_MerchantFrame.GetNumJunkItems()==0, 'closed without a host')")
        .unwrap();
    let mut vendor = Vendor::default();
    script.hosted(&mut vendor, |script| {
        script
            .run(
                "assert(C_MerchantFrame.GetNumJunkItems()==1, 'junk '..C_MerchantFrame.GetNumJunkItems())
                 assert(C_MerchantFrame.GetBuybackItemID(1)==40); C_MerchantFrame.SellAllJunkItems()",
            )
            .unwrap();
        script.tick(0.1);
        script.tick(0.1);
    });
    assert_eq!(vendor.sold, vec![101]);
    assert!(script.errors().is_empty(), "{:?}", script.errors());
}

#[test]
fn spell_known_checks_the_visible_book_and_marker_queries_use_live_units() {
    use benilla_ui::script::{
        pfui_host::units, PetBookState, SpellBookState, SpellSlotView, UnitState,
    };
    let mut script = extended();
    script.set_spellbook(SpellBookState {
        slots: vec![SpellSlotView {
            spell_id: 133,
            ..Default::default()
        }],
        ..Default::default()
    });
    script.set_pet_book(PetBookState {
        slots: vec![SpellSlotView {
            spell_id: 2649,
            ..Default::default()
        }],
        ..Default::default()
    });
    units::set_known_spells(script.lua(), [133, 1000]);
    units::set_unit_states(
        script.lua(),
        [
            (
                5,
                UnitState {
                    guid: 5,
                    exists: true,
                    has_object: true,
                    name: Some("Mob".into()),
                    raid_target: 8,
                    ..Default::default()
                },
                6,
            ),
            (
                6,
                UnitState {
                    guid: 6,
                    exists: true,
                    has_object: true,
                    name: Some("Other".into()),
                    ..Default::default()
                },
                0,
            ),
        ],
    );
    script
        .run(
            r#"
        assert(IsSpellKnown(133) and not IsSpellKnown(1000) and not IsSpellKnown(2649))
        assert(IsSpellKnown(2649,true) and not IsSpellKnown(0))
        assert(UnitName('mark8')=='Mob' and UnitGUID('mark8')=='0x0000000000000005')
        assert(UnitName('mark8target')=='Other' and not UnitExists('mark7'))
        local t={a=2,[40]=3,4}; assert(table.wipe(t)==t and next(t)==nil)
    "#,
        )
        .unwrap();
    units::set_unit_states(script.lua(), []);
    script
        .run("assert(not UnitExists('mark8') and not UnitExists('mark9'))")
        .unwrap();
}

#[test]
fn new_items_exclude_login_inventory_and_follow_guids_across_sorting() {
    use benilla_ui::script::{
        pfui_host::items::{self, Instance},
        UnitGuids,
    };
    let mut script = extended();
    script.set_unit_guids(&UnitGuids {
        player: 3,
        ..Default::default()
    });
    let item = |guid| Instance {
        guid,
        item_id: 38,
        count: 1,
        uses: 1,
        name: None,
    };
    script.run("newEvents=0; local f=CreateFrame('Frame'); f:RegisterEvent('BAG_NEW_ITEMS_UPDATED'); f:SetScript('OnEvent',function() newEvents=newEvents+1 end)").unwrap();
    items::set_instances(script.lua(), [((0, 1), item(101))]);
    script.tick(2.);
    items::set_instances(script.lua(), [((0, 1), item(101)), ((0, 2), item(102))]);
    script
        .run("assert(newEvents==1 and not C_NewItems.IsNewItem(0,1) and C_NewItems.IsNewItem(0,2))")
        .unwrap();
    items::set_instances(
        script.lua(),
        [
            ((0, 1), item(102)),
            ((0, 2), item(101)),
            ((-1, 1), item(103)),
        ],
    );
    script.run("assert(newEvents==1 and C_NewItems.IsNewItem(0,1) and not C_NewItems.IsNewItem(-1,1)); C_NewItems.RemoveNewItem(0,1); assert(newEvents==2)").unwrap();
    items::set_instances(script.lua(), [((0, 1), item(101)), ((0, 2), item(102))]);
    script.run("assert(not C_NewItems.IsNewItem(0,2) and not C_NewItems.IsNewItem(nil,'bad')); C_NewItems.ClearAll(); assert(newEvents==2)").unwrap();
    items::set_instances(
        script.lua(),
        [
            ((0, 1), item(101)),
            ((0, 2), item(102)),
            ((0, 3), item(104)),
        ],
    );
    script
        .run("assert(newEvents==3 and C_NewItems.IsNewItem(0,3))")
        .unwrap();
    items::set_instances(script.lua(), [((0, 1), item(101)), ((0, 2), item(102))]);
    script
        .run("assert(newEvents==4 and not C_NewItems.IsNewItem(0,3))")
        .unwrap();
    script.set_unit_guids(&UnitGuids {
        player: 4,
        ..Default::default()
    });
    items::set_instances(script.lua(), [((0, 1), item(105))]);
    script.run("assert(not C_NewItems.IsNewItem(0,1))").unwrap();
    assert!(script.errors().is_empty(), "{:?}", script.errors());
}

#[test]
fn nameplate_lifecycle_keeps_gaps_and_readable_removal_tokens() {
    use benilla_ui::script::{pfui_host::units, PlateGeometry, PlateState, UnitState};
    let mut script = extended();
    script.set_screen_size(1024., 768.);
    script.run(r#"
        WorldFrame=CreateFrame('WorldFrame','WorldFrame'); WorldFrame:SetAllPoints()
        created=0; added=0; removed=0; plates={}
        local f=CreateFrame('Frame')
        f:RegisterEvent('NAME_PLATE_CREATED'); f:RegisterEvent('NAME_PLATE_UNIT_ADDED'); f:RegisterEvent('NAME_PLATE_UNIT_REMOVED')
        f:SetScript('OnEvent',function(self,e,unit)
            assert(event==e and arg1==unit)
            if e=='NAME_PLATE_CREATED' then
                created=created+1; assert(unit:GetObjectType()=='Button'); plates[created]=unit
            elseif e=='NAME_PLATE_UNIT_ADDED' then
                added=added+1; assert(UnitExists(unit) and C_NamePlate.GetNamePlateForUnit(unit))
            else
                removed=removed+1; assert(UnitGUID(unit) and C_NamePlate.GetNamePlateForUnit(unit))
                removedGuid=UnitGUID(unit)
            end
        end)
        assert(not UnitExists('nameplate99') and UnitGUID('nameplate0')==nil)
    "#).unwrap();
    script.resolve();
    let geometry = PlateGeometry {
        width: 128.,
        height: 32.,
        bar_off_x: 4.,
        bar_off_y: 4.,
        bar_width: 103.,
        bar_height: 9.,
        level_off_x: 12.,
        level_off_y: 9.,
        skull_size: 13.,
        raid_size: 26.,
        name_height: 13.,
        level_height: 11.,
        shadow_offset: 1.,
    };
    let plate = |key| PlateState {
        key,
        top_centre: (500., 400.),
        health: 40.,
        max_health: 50.,
        bar_colour: [1., 0., 0.],
        name: format!("Mob{key}"),
        level: Some(8),
        skull: false,
        level_colour: [1., 1., 0.],
        raid_icon: None,
        alpha: 255,
        lit: false,
        hovered: false,
    };
    units::set_unit_states(
        script.lua(),
        (5..=7).map(|guid| {
            (
                guid,
                UnitState {
                    guid,
                    exists: true,
                    has_object: true,
                    name: Some(format!("Mob{guid}")),
                    health: 40,
                    max_health: 50,
                    subtitle: Some("Trainer".into()),
                    ..Default::default()
                },
                6,
            )
        }),
    );
    script.sync_nameplates(geometry, &[plate(5), plate(6)]);
    script
        .run(
            r#"
        assert(created==2 and added==2 and removed==0)
        assert(UnitName('nameplate1')=='Mob5' and UnitName('nameplate2')=='Mob6')
        assert(UnitSubName('nameplate1')=='Trainer' and UnitName('nameplate1target')=='Mob6')
        assert(C_NamePlate.GetNamePlateForUnit('nameplate1')==plates[1])
    "#,
        )
        .unwrap();
    script.sync_nameplates(geometry, &[plate(6)]);
    script
        .run(
            r#"
        assert(removed==1 and removedGuid=='0x0000000000000005')
        assert(UnitGUID('nameplate1')==nil and UnitName('nameplate2')=='Mob6')
        assert(C_NamePlate.GetNamePlateForUnit('nameplate1')==nil)
    "#,
        )
        .unwrap();
    script.sync_nameplates(geometry, &[plate(7), plate(6)]);
    script.run("assert(created==2 and added==3 and UnitName('nameplate1')=='Mob7' and C_NamePlate.GetNamePlateForUnit('nameplate1')==plates[1])").unwrap();
    script.retire_nameplates();
    script
        .run("assert(removed==3 and UnitGUID('nameplate1')==nil and UnitGUID('nameplate2')==nil)")
        .unwrap();
    assert!(script.errors().is_empty(), "{:?}", script.errors());
    assert!(UiScript::new()
        .unwrap()
        .eval::<bool>("return IsInGroup==nil and UnitSubName==nil and C_NamePlate==nil")
        .unwrap());
}

#[test]
fn group_queries_use_party_and_raid_rosters() {
    use benilla_ui::script::{PartyMemberInfo, PartyState, RaidMemberInfo};
    let mut script = extended();
    script
        .run("assert(not IsInGroup() and not IsInRaid(2))")
        .unwrap();
    script.set_party(PartyState {
        members: vec![PartyMemberInfo {
            guid: 7,
            name: "Other".into(),
        }],
        ..Default::default()
    });
    script
        .run("assert(IsInGroup(1) and not IsInRaid())")
        .unwrap();
    script.set_party(PartyState {
        raid: vec![RaidMemberInfo {
            guid: 3,
            ..Default::default()
        }],
        ..Default::default()
    });
    script.run("assert(IsInGroup() and IsInRaid())").unwrap();
}

#[test]
fn focus_and_guid_queries_use_live_snapshots_and_restore_stock_aliases_on_errors() {
    use benilla_ui::script::{
        pfui_host::units, ScriptCall, SelectionRequest, UnitGuids, UnitState,
    };
    let mut script = extended();
    script.set_unit_guids(&UnitGuids {
        player: 3,
        target: 5,
        npc: 99,
        mouseover: 77,
        held: std::collections::HashMap::from([(77, 0)]),
        ..Default::default()
    });
    let original = UnitState {
        guid: 99,
        exists: true,
        name: Some("Original".into()),
        ..Default::default()
    };
    script.set_unit("npc", Some(original));
    let mob = UnitState {
        guid: 5,
        exists: true,
        has_object: true,
        name: Some("Mob".into()),
        health: 40,
        max_health: 50,
        level: 8,
        reaction: 3,
        ..Default::default()
    };
    let other = UnitState {
        guid: 6,
        exists: true,
        has_object: true,
        name: Some("Other".into()),
        health: 30,
        max_health: 40,
        level: 7,
        reaction: 3,
        ..Default::default()
    };
    units::set_unit_states(script.lua(), [(5, mob.clone(), 6), (6, other.clone(), 5)]);
    script.run(r#"
        assert(UnitName('focus')==nil and UnitExists('focus')==nil and UnitHealth('focus')==0)
        focusChanges=0
        local frame=CreateFrame('Frame'); frame:RegisterEvent('PLAYER_FOCUS_CHANGED')
        frame:SetScript('OnEvent',function() focusChanges=focusChanges+1 end)
        FocusUnit()
        assert(focusChanges==1 and UnitGUID('focus')=='0x0000000000000005')
        assert(UnitName('focus')=='Mob' and UnitHealth('focus')==40 and UnitLevel('focus')==8)
        assert(UnitPVPName('focus')=='Mob' and UnitPVPRank('focus')==0)
        assert(UnitName('focustarget')=='Other' and UnitName('focustargettarget')=='Mob')
        assert(UnitGUID('npc')=='0x0000000000000063' and UnitName('npc')=='Original')
        assert(UnitGUID('mouseover')=='0x000000000000004D')
        assert(UnitIsUnit('focus','0x00000005')==1 and not UnitIsUnit('focus','0x00000006'))
        FocusUnit('0x0000000000000005'); assert(focusChanges==1)
        local success=pcall(UnitIsUnit,'focus','unrecognized')
        assert(not success)
        assert(UnitGUID('npc')=='0x0000000000000063' and UnitName('npc')=='Original')
        TargetUnit('focus'); ClearTarget()
        healthEvents=0; local updates=CreateFrame('Frame'); updates:RegisterUnitEvent('UNIT_HEALTH','focus')
        updates:SetScript('OnEvent',function(self,e,unit) assert(unit=='focus'); healthEvents=healthEvents+1 end)
    "#).unwrap();
    let calls = script.take_script_calls();
    assert_eq!(
        calls[0],
        ScriptCall::Select {
            request: SelectionRequest::PfuiGuid(5),
            may_cast: true
        }
    );
    assert!(matches!(calls[1], ScriptCall::ClearTarget));
    let mut changed = mob;
    changed.health = 25;
    units::set_unit_states(script.lua(), [(5, changed, 6), (6, other, 5)]);
    script.run("assert(healthEvents==1 and UnitHealth('focus')==25) ClearFocus() assert(focusChanges==2 and UnitName('focus')==nil)").unwrap();
    let stock = UiScript::new().unwrap();
    assert!(stock.run("UnitName('focus')").is_err());
    assert!(script.errors().is_empty(), "{:?}", script.errors());
}

#[test]
fn typed_power_uses_each_resource_and_local_guid_checks_do_not_guess() {
    use benilla_ui::script::{
        pfui_host::units::{self, Power},
        UnitGuids,
    };
    let mut script = extended();
    script.set_unit_guids(&UnitGuids {
        player: 3,
        target: 4,
        ..Default::default()
    });
    units::set_powers(
        script.lua(),
        [(
            3,
            Power {
                kind: 3,
                current: [Some(130), Some(75), None, Some(70), None],
                maximum: [Some(200), Some(1000), None, Some(100), None],
            },
        )],
    );
    script
        .run(
            r#"
        assert(UnitPower('player')==70 and UnitPowerMax('player')==100)
        assert(UnitPower('player',Enum.PowerType.Mana)==130 and UnitPowerMax('player',0)==200)
        assert(UnitPower('player',1)==7 and UnitPower('player',1,true)==75)
        assert(UnitPowerMax('player',1)==100 and UnitPowerMax('player',1,true)==1000)
        assert(UnitPower('0x0000000000000003',0)==130)
        assert(UnitPower('player',-1)==0 and UnitPowerMax('player',5)==0)
        assert(IsPlayerGuid('player') and IsPlayerGuid('0x0000000000000003'))
        assert(not IsPlayerGuid('target') and not IsPlayerGuid(nil) and not IsPlayerGuid('bad'))
    "#,
        )
        .unwrap();
    units::set_powers(script.lua(), []);
    script.run("assert(UnitPower('player',0)==0)").unwrap();
    script.set_unit_guids(&UnitGuids::default());
    script
        .run("assert(not IsPlayerGuid('0x0000000000000000'))")
        .unwrap();
}

#[test]
fn creature_ids_prefer_live_templates_and_never_decode_pet_numbers_as_entries() {
    use benilla_ui::script::pfui_host::units;
    let script = extended();
    script
        .lua()
        .globals()
        .set(
            "CreatureID",
            script.lua().create_function(units::creature_id).unwrap(),
        )
        .unwrap();
    let guid = 0xf130_0000_7b00_002a;
    let pet = 0xf140_0000_7b00_002a;
    script
        .run(
            r#"
        assert(CreatureID('0xF13000007B00002A')==123)
        assert(CreatureID('0xF14000007B00002A')==nil)
        assert(CreatureID('0x400000007B00002A')==nil and CreatureID('0x0000000000000003')==nil)
        assert(CreatureID(nil)==nil and CreatureID(123)==nil and CreatureID('0xF130bad')==nil)
    "#,
        )
        .unwrap();
    units::set_creature_ids(script.lua(), [(guid, 456), (pet, 789)]);
    script.run("assert(CreatureID('0xF13000007B00002A')==456 and CreatureID('0xF14000007B00002A')==789)").unwrap();
    units::set_creature_ids(script.lua(), []);
    script.run("assert(CreatureID('0xF13000007B00002A')==123 and CreatureID('0xF14000007B00002A')==nil)").unwrap();
}

#[test]
fn legacy_spell_metadata_preserves_tuple_positions_and_resolves_both_books() {
    use super::spell::{Catalog, Spell};
    use benilla_ui::script::{PetBookState, SpellBookState, SpellSlotView};
    let mut script = extended();
    let slot = |spell_id, name: &str, rank: &str, passive| SpellSlotView {
        spell_id,
        name: name.into(),
        rank: Some(rank.into()),
        passive,
        ..Default::default()
    };
    let slots = vec![
        slot(133, "Fireball", "Rank 1", false),
        slot(143, "Fireball", "Rank 2", false),
        slot(145, "Fireball", "Rank 10", false),
        slot(144, "Fireball", "Rank 3", false),
        slot(200, "Passive", "", true),
    ];
    script.set_spellbook(SpellBookState {
        slots: slots.clone(),
        ..Default::default()
    });
    let pet_slots = vec![
        slot(300, "Growl", "Rank 1", false),
        slot(301, "Fireball", "Rank 20", false),
    ];
    script.set_pet_book(PetBookState {
        slots: pet_slots.clone(),
        ..Default::default()
    });
    let records = slots
        .into_iter()
        .chain(pet_slots)
        .chain([slot(999, "Unlearned", "", false)])
        .map(|slot| {
            (
                slot.spell_id,
                Spell {
                    name: slot.name,
                    rank: slot.rank.unwrap(),
                    icon: (slot.spell_id != 999).then(|| "Icon".into()),
                    dispel: 0,
                    attributes: 0,
                    reagents: vec![],
                    effects: vec![],
                    cast_time: 1500,
                    min_range: 5.0,
                    max_range: 35.0,
                    cost: 30,
                    power_type: 0,
                    funnel: true,
                    swing: None,
                },
            )
        })
        .collect();
    Catalog(records).install(script.lua()).unwrap();
    script.run(r#"
      local name,rank,icon,cost,funnel,power,time,min,max,id=GetSpellInfo('999')
      assert(select('#',GetSpellInfo(999))==10)
      assert(name=='Unlearned' and rank=='' and icon==nil and cost==30 and funnel==true)
      assert(power==0 and time==1500 and min==5 and max==35 and id==999)
      assert(select(10,GetSpellInfo('Fireball'))==145)
      assert(select(10,GetSpellInfo('Fireball (Rank 2)'))==143)
      assert(select(10,GetSpellInfo('Passive'))==200)
      assert(select(10,GetSpellInfo('Growl'))==300)
      assert(select(10,GetSpellInfo(1,'spell'))==133)
      assert(select(10,GetSpellInfo('1','pet'))==300)
      assert(select(10,GetSpellInfo('|cfff00|Hspell:999|h[Unlearned]|h|r'))==999)
      assert(select(10,GetSpellInfo('spell:999'))==999)
      assert(GetSpellInfo('fireball')==nil and GetSpellInfo('Unlearned')==nil)
      assert(GetSpellInfo('Fireball(Rank 4)')==nil and select(10,GetSpellInfo('Fireball(Rank 20)'))==301)
      assert(GetSpellInfo(6,'spell')==nil and GetSpellInfo(3,'pet')==nil and GetSpellInfo(1,'invalid')==nil)
      assert(GetSpellInfo(0)==nil and GetSpellInfo(-1)==nil and GetSpellInfo(133.5)==nil)
      assert(GetSpellInfo(0,'spell')==nil and GetSpellInfo(1.5,'spell')==nil)
      assert(GetSpellInfo(0/0)==nil and GetSpellInfo(math.huge)==nil and GetSpellInfo({})==nil)
      assert(select('#',GetSpellInfo(404))==1 and GetSpellInfo(404)==nil)
    "#).unwrap();
    let stock = UiScript::new().unwrap();
    stock.run("assert(GetSpellInfo==nil)").unwrap();
}

#[test]
fn caster_stats_use_school_indices_and_current_unit_speed_snapshots() {
    use benilla_ui::script::{
        pfui_host::{player_stats, units},
        UnitGuids,
    };
    let mut script = extended();
    script.run("for school=1,7 do assert(GetSpellBonusDamage(school)==0) end; assert(UnitSpellHaste('player')==0)").unwrap();
    player_stats::set_damage(script.lua(), [12, 0, 30, -8, 50, 60, 70]);
    script.set_unit_guids(&UnitGuids {
        player: 3,
        target: 4,
        ..Default::default()
    });
    units::set_unit_states(
        script.lua(),
        [(
            5,
            benilla_ui::script::UnitState {
                guid: 5,
                exists: true,
                has_object: true,
                ..Default::default()
            },
            0,
        )],
    );
    script.run("FocusUnit('0x0000000000000005')").unwrap();
    player_stats::set_cast_speeds(script.lua(), [(3, 1.0), (4, 0.5), (5, 2.0)]);
    script
        .run(
            r#"
      local expected={12,0,30,-8,50,60,70}
      for school=1,7 do assert(GetSpellBonusDamage(school)==expected[school]) end
      assert(GetSpellBonusDamage('3')==30)
      assert(not pcall(GetSpellBonusDamage) and not pcall(GetSpellBonusDamage,0))
      assert(not pcall(GetSpellBonusDamage,8) and not pcall(GetSpellBonusDamage,1.5))
      assert(not pcall(GetSpellBonusDamage,0/0) and not pcall(GetSpellBonusDamage,math.huge))
      assert(not pcall(GetSpellBonusDamage,{}) and not pcall(GetSpellBonusDamage,'Fire'))
      assert(UnitSpellHaste('player')==0 and UnitSpellHaste('target')==100)
      assert(UnitSpellHaste('focus')==-50 and UnitSpellHaste('0x0000000000000005')==-50)
      assert(UnitSpellHaste('party4')==0 and UnitSpellHaste('invalid')==0)
      assert(UnitSpellHaste(nil)==0 and UnitSpellHaste({})==0)
    "#,
        )
        .unwrap();
    for speed in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        player_stats::set_cast_speeds(script.lua(), [(3, speed)]);
        script.run("assert(UnitSpellHaste('player')==0 and UnitSpellHaste('focus')==0 and UnitSpellHaste('target')==0)").unwrap();
    }
    player_stats::set_damage(script.lua(), [0; 7]);
    script.run("assert(GetSpellBonusDamage(7)==0)").unwrap();
    let stock = UiScript::new().unwrap();
    stock
        .run("assert(GetSpellBonusDamage==nil and UnitSpellHaste==nil)")
        .unwrap();
}

#[test]
fn structured_spell_metadata_and_aura_lookup_include_hidden_known_spells() {
    use super::spell::{Catalog, Spell};
    use benilla_ui::script::pfui_host::units;
    let script = extended();
    let effects = vec![std::collections::BTreeMap::from([
        ("auraName", 254.0),
        ("effect", 6.0),
    ])];
    let spell = Spell {
        name: "Fireball".into(),
        rank: "Rank 1".into(),
        icon: Some("Icon".into()),
        dispel: 0,
        attributes: 0x40,
        reagents: vec![],
        effects,
        cast_time: 1500,
        min_range: 0.0,
        max_range: 35.0,
        cost: 30,
        power_type: 0,
        funnel: false,
        swing: None,
    };
    Catalog(std::collections::BTreeMap::from([
        (1, spell.clone()),
        (2, spell),
    ]))
    .install(script.lua())
    .unwrap();
    units::set_known_spells(script.lua(), [2, 1, 999]);
    script.run(r#"
        local info=C_Spell.GetSpellInfo(1)
        assert(info.name=='Fireball' and info.rank=='Rank 1' and info.spellID==1)
        assert(info.castTime==1500 and info.maxRange==35 and info.minRange==0 and info.iconID=='Icon')
        assert(info.cost==30 and info.powerType==0 and not info.isFunnel)
        assert(C_Spell.GetSpellInfo(999)==nil and C_Spell.GetSpellInfo(0)==nil)
        local known=C_SpellBook.GetPlayerSpellsByAura(254)
        assert(#known==2 and known[1]==1 and known[2]==2)
        assert(#C_SpellBook.GetPlayerSpellsByAura(0)==0)
    "#).unwrap();
    units::set_known_spells(script.lua(), [2]);
    script
        .run(
            "local known=C_SpellBook.GetPlayerSpellsByAura(254); assert(#known==1 and known[1]==2)",
        )
        .unwrap();
}

#[test]
fn equipment_buttons_drag_swap_and_follow_set_edits_without_virtual_wire_actions() {
    use benilla_ui::script::{ActionSlot, SpellBookState, SpellSlotView, UnitState};
    let mut script = extended();
    script.set_realm_name("Realm");
    script.set_unit(
        "player",
        Some(UnitState {
            exists: true,
            name: Some("Test".into()),
            ..Default::default()
        }),
    );
    script.set_action(
        1,
        Some(ActionSlot {
            action: 133,
            kind: 0,
            ..Default::default()
        }),
    );
    script.set_spellbook(SpellBookState {
        slots: vec![
            SpellSlotView {
                spell_id: 133,
                name: "Fireball".into(),
                ..Default::default()
            },
            SpellSlotView {
                spell_id: 200,
                name: "Passive".into(),
                passive: true,
                ..Default::default()
            },
        ],
        ..Default::default()
    });
    script.run(r#"
      local api=C_EquipmentSet
      one=api.CreateEquipmentSet('First','INV_Shirt_01')
      api.PickupEquipmentSet(404); assert(GetCursorInfo()==nil)
      transitions={}; local f=CreateFrame('Frame'); f:RegisterEvent('CURSOR_CHANGED')
      f:SetScript('OnEvent',function(self,event,...) transitions[#transitions+1]=SafePack(...) end)
      api.PickupEquipmentSet(one)
      local kind,name=GetCursorInfo(); assert(kind=='equipmentset' and name=='First' and select('#',GetCursorInfo())==2)
      assert(not CursorHasItem() and not CursorHasSpell())
      assert(PlaceAction(1) and HasAction(1))
      assert(GetActionInfo(1)=='equipmentset' and GetActionText(1)=='First')
      assert(GetActionTexture(1)=='Interface\\Icons\\INV_Shirt_01')
      ClearCursor()
    "#).unwrap();
    assert_eq!(script.take_action_sets(), [(1, 0)]);
    script.tick(0.1);
    script.run(r#"
      assert(#transitions==3)
      assert(transitions[1].n==4 and transitions[1][1]==nil and transitions[1][2]==12 and transitions[1][3]==0)
      assert(transitions[2][2]==3 and transitions[2][3]==12 and transitions[2][4]==one)
      assert(transitions[3][1]==1 and transitions[3][2]==0 and transitions[3][3]==3 and transitions[3][4]==133)
      assert(PickupAction(1)); assert(not HasAction(1) and GetCursorInfo()=='equipmentset')
      assert(PlaceAction(2) and GetCursorInfo()==nil)
      C_EquipmentSet.ModifyEquipmentSet(one,'Renamed')
      C_EquipmentSet.SaveEquipmentSet(one,'Interface\\Icons\\INV_Shirt_02')
      assert(GetActionText(2)=='Renamed' and GetActionTexture(2)=='Interface\\Icons\\INV_Shirt_02')
      local usable,oom=IsUsableAction(2); assert(usable==1 and oom==nil)
      assert(GetActionCount(2)==0 and ActionHasRange(2)==nil and IsCurrentAction(2)==nil)
      local start,duration,enable=GetActionCooldown(2); assert(start==0 and duration==0 and enable==1)
      PickupSpell(1,'spell'); assert(PlaceAction(2))
      assert(GetActionInfo(2)=='spell' and GetCursorInfo()=='equipmentset')
      assert(PlaceAction(3) and GetCursorInfo()==nil)
      PickupSpell(2,'spell'); assert(not PlaceAction(3))
      assert(GetActionInfo(3)=='equipmentset' and CursorHasSpell())
      ClearCursor()
      local two=C_EquipmentSet.CreateEquipmentSet('Second')
      C_EquipmentSet.PickupEquipmentSet(two); assert(PlaceAction(3))
      local _,displaced=GetCursorInfo(); assert(displaced=='Renamed' and GetActionText(3)=='Second')
      C_EquipmentSet.DeleteEquipmentSet(one); assert(GetCursorInfo()==nil and HasAction(3))
      C_EquipmentSet.DeleteEquipmentSet(two); assert(not HasAction(3))
    "#).unwrap();
    assert_eq!(script.take_action_sets(), [(2, 133)]);
    assert!(script.errors().is_empty(), "{:?}", script.errors());
}

#[test]
fn equipment_button_bindings_reload_by_character_and_yield_to_server_changes() {
    use benilla_ui::script::{ActionSlot, UnitState};
    let setup = |script: &mut UiScript, name: &str| {
        script.set_realm_name("Realm");
        script.set_unit(
            "player",
            Some(UnitState {
                exists: true,
                name: Some(name.into()),
                ..Default::default()
            }),
        );
    };
    let mut script = extended();
    setup(&mut script, "Test");
    script.run("local id=C_EquipmentSet.CreateEquipmentSet('Set'); C_EquipmentSet.PickupEquipmentSet(id); assert(PlaceAction(10))").unwrap();
    let saved = script.saved_variables_bytes();
    let mut reloaded = extended();
    setup(&mut reloaded, "Test");
    reloaded.run_chunk(&saved).unwrap();
    reloaded
        .run("assert(GetActionInfo(10)=='equipmentset' and GetActionText(10)=='Set')")
        .unwrap();
    setup(&mut reloaded, "Other");
    reloaded
        .run("assert(not HasAction(10) and C_EquipmentSet.GetNumEquipmentSets()==0)")
        .unwrap();
    setup(&mut reloaded, "Test");
    reloaded
        .run("assert(GetActionInfo(10)=='equipmentset')")
        .unwrap();
    reloaded.set_action(
        10,
        Some(ActionSlot {
            action: 133,
            kind: 0,
            ..Default::default()
        }),
    );
    reloaded.run("assert(GetActionInfo(10)=='spell')").unwrap();
    reloaded.set_action(10, None);
    reloaded.run("assert(not HasAction(10))").unwrap();
    reloaded.set_realm_name("Other Realm");
    reloaded.run("assert(not HasAction(10))").unwrap();
    assert!(reloaded.take_action_sets().is_empty());
}

#[test]
fn equipment_buttons_use_the_real_swap_queue_and_refuse_locked_items() {
    use benilla_ui::script::{
        pfui_host::items::{set_instances, Instance},
        InvSlotView, UnitState, EQUIPMENT_BAG,
    };
    let mut script = extended();
    script.set_realm_name("Realm");
    script.set_unit(
        "player",
        Some(UnitState {
            exists: true,
            name: Some("Test".into()),
            ..Default::default()
        }),
    );
    let instance = || Instance {
        guid: 0x4000_0000_0000_0001,
        item_id: 38,
        count: 1,
        uses: 0,
        name: Some("Shirt".into()),
    };
    set_instances(script.lua(), [((EQUIPMENT_BAG, 4), instance())]);
    script.run(r#"
      local id=C_EquipmentSet.CreateEquipmentSet('Shirt')
      C_EquipmentSet.PickupEquipmentSet(id); assert(PlaceAction(4))
      wearEvents=0; errors=0
      local f=CreateFrame('Frame'); f:RegisterEvent('WEAR_EQUIPMENT_SET'); f:RegisterEvent('UI_ERROR_MESSAGE')
      f:SetScript('OnEvent',function(self,event,id)
        if event=='WEAR_EQUIPMENT_SET' then wearEvents=wearEvents+1; assert(id==1) else errors=errors+1 end
      end)
      tip=CreateFrame('GameTooltip','EquipmentActionTip',nil,'TestTooltipTemplate'); tip:SetAction(4)
      assert(EquipmentActionTipTextLeft1:GetText()=='Shirt')
    "#).unwrap();
    set_instances(script.lua(), [((0, 1), instance())]);
    let mut bag = benilla_ui::script::ContainerState {
        num_slots: 2,
        ..Default::default()
    };
    bag.slots.insert(
        1,
        benilla_ui::script::ContainerSlot {
            item_id: 38,
            guid: 0x4000_0000_0000_0026,
            ..Default::default()
        },
    );
    script.set_container(0, Some(bag.clone()));
    // The lock is the item's own now, by guid.
    script.lock_item(0x4000_0000_0000_0026);
    script
        .run("assert(IsUsableAction(4)==nil); UseAction(4); assert(wearEvents==1 and errors==1)")
        .unwrap();
    assert!(script.take_container_moves().is_empty());
    script.unlock_item(0x4000_0000_0000_0026);
    script.set_container(0, Some(bag));
    let mut slots: benilla_ui::script::InventorySlots = Default::default();
    slots[4] = Some(InvSlotView {
        item_id: 0,
        ..Default::default()
    });
    script.set_inventory_slots(slots);
    script
        .run("assert(IsUsableAction(4)==1); UseAction(4); assert(wearEvents==2 and errors==1)")
        .unwrap();
    assert_eq!(
        script.take_container_moves(),
        [benilla_ui::script::ContainerMove {
            src_bag: 0,
            src_slot: 1,
            dst_bag: EQUIPMENT_BAG,
            dst_slot: 4,
            count: None
        }]
    );
    assert!(script.take_action_uses().is_empty());
    assert!(script.errors().is_empty(), "{:?}", script.errors());
}

#[test]
fn equipment_sets_track_duplicate_item_guids_wait_for_ack_and_persist() {
    use benilla_ui::script::{
        pfui_host::items::{set_instances, Instance},
        ContainerMove, UnitState, EQUIPMENT_BAG,
    };
    let setup = |script: &mut UiScript| {
        script.set_realm_name("Realm");
        script.set_unit(
            "player",
            Some(UnitState {
                exists: true,
                name: Some("Test".into()),
                ..Default::default()
            }),
        );
    };
    let rings = |swapped: bool| {
        [
            (
                11,
                if swapped {
                    0x4000_0000_0000_0002
                } else {
                    0x4000_0000_0000_0001
                },
            ),
            (
                12,
                if swapped {
                    0x4000_0000_0000_0001
                } else {
                    0x4000_0000_0000_0002
                },
            ),
        ]
        .map(|(slot, guid)| {
            (
                (EQUIPMENT_BAG, slot),
                Instance {
                    guid,
                    item_id: 7,
                    count: 1,
                    uses: 1,
                    name: None,
                },
            )
        })
    };
    let mut script = extended();
    setup(&mut script);
    set_instances(script.lua(), rings(false));
    script.run(r#"
        equipmentEvents={}
        local frame=CreateFrame('Frame')
        frame:RegisterEvent('EQUIPMENT_SWAP_PENDING'); frame:RegisterEvent('EQUIPMENT_SWAP_FINISHED')
        frame:SetScript('OnEvent',function(self,e,a,b) equipmentEvents[#equipmentEvents+1]={e,a,b} end)
        C_EquipmentSet.IgnoreSlotForSave(19)
        setID=C_EquipmentSet.CreateEquipmentSet('Rings')
        assert(setID==1 and C_EquipmentSet.CreateEquipmentSet('Rings')==nil)
        C_EquipmentSet.ClearIgnoredSlotsForSave()
        assert(C_EquipmentSet.GetIgnoredSlots(setID)[1]==19)
        local name,icon,id,equipped,total,worn,carried,missing,ignored=C_EquipmentSet.GetEquipmentSetInfo(setID)
        assert(name=='Rings' and equipped and total==2 and worn==2 and carried==0 and missing==0 and ignored==1)
        assert(C_EquipmentSet.GetItemLocations(setID)[19]==1)
        C_EquipmentSet.ModifyEquipmentSet(setID,'Gear')
        assert(C_EquipmentSet.GetEquipmentSetID('Gear')==setID and C_EquipmentSet.GetEquipmentSetID('gear')==nil)
    "#).unwrap();
    let saved = script.saved_variables_bytes();
    assert!(String::from_utf8_lossy(&saved).contains("0x4000000000000001"));
    set_instances(script.lua(), rings(true));
    script
        .run(
            r#"
        local _,_,_,equipped,_,worn,carried=C_EquipmentSet.GetEquipmentSetInfo(setID)
        assert(not equipped and worn==0 and carried==2)
        assert(C_EquipmentSet.UseEquipmentSet(setID))
        assert(#equipmentEvents==1 and equipmentEvents[1][1]=='EQUIPMENT_SWAP_PENDING')
        assert(C_EquipmentSet.GetItemIDs(setID)[11]==7 and C_EquipmentSet.GetItemIDs(setID)[12]==7)
    "#,
        )
        .unwrap();
    assert_eq!(
        script.take_container_moves(),
        [ContainerMove {
            src_bag: EQUIPMENT_BAG,
            src_slot: 12,
            dst_bag: EQUIPMENT_BAG,
            dst_slot: 11,
            count: None
        }]
    );
    script.tick(0.1);
    assert!(script.take_container_moves().is_empty());
    script.run("assert(#equipmentEvents==1)").unwrap();
    // The server's one atomic ring swap resolves both target slots.
    set_instances(script.lua(), rings(false));
    script.tick(0.1);
    assert!(script.take_container_moves().is_empty());
    script.run("assert(#equipmentEvents==2 and equipmentEvents[2][2]==1 and equipmentEvents[2][3]==setID)").unwrap();
    // A failed transaction cannot report a completed set swap.
    set_instances(script.lua(), rings(true));
    script
        .run("assert(C_EquipmentSet.UseEquipmentSet(setID))")
        .unwrap();
    assert_eq!(script.take_container_moves().len(), 1);
    script.tick(10.1);
    script
        .run(
            "assert(equipmentEvents[4][1]=='EQUIPMENT_SWAP_FINISHED' and equipmentEvents[4][2]==0)",
        )
        .unwrap();
    let mut reloaded = extended();
    setup(&mut reloaded);
    reloaded.run_chunk(&saved).unwrap();
    set_instances(reloaded.lua(), rings(false));
    reloaded
        .run(
            r#"
        assert(C_EquipmentSet.GetNumEquipmentSets()==1)
        assert(select(4,C_EquipmentSet.GetEquipmentSetInfo(1)))
        C_EquipmentSet.DeleteEquipmentSet(1)
        assert(C_EquipmentSet.CreateEquipmentSet('Next')==2)
    "#,
        )
        .unwrap();
    reloaded.set_realm_name("Different Realm");
    reloaded
        .run("assert(C_EquipmentSet.GetNumEquipmentSets()==0)")
        .unwrap();
    assert!(script.errors().is_empty(), "{:?}", script.errors());
}

#[test]
fn string_icon_macros_use_the_stock_table_and_persistence_edges() {
    let mut script = extended();
    script.set_macro_icons(vec!["Interface\\Icons\\Spell_Nature_Heal".into()]);
    script
        .run(
            r#"
        assert(C_Macro.CreateMacro('Item', 'INV_Sword_25', '/wave', false)==1)
        assert(C_Macro.CreateMacro('Character', 'Interface\\Icons\\INV_Potion_51', nil, true)==19)
        local name, icon, body=GetMacroInfo(1)
        assert(name=='Item' and icon=='Interface\\Icons\\INV_Sword_25' and body=='/wave')
        assert(C_Macro.EditMacro('item', nil, 'INV_Sword_26', nil)==1)
        assert(C_Macro.EditMacro(1, nil, 99, '/bow')==1)
        name, icon, body=GetMacroInfo(1)
        assert(icon=='Interface\\Icons\\INV_Sword_26' and body=='/bow')
        assert(C_Macro.EditMacro(99)==nil)
        assert(C_Macro.EditMacro('Missing')==nil)
        -- The numeric-index ABI remains separate, using only the actual chooser list.
        assert(CreateMacro('Stock',1,'/dance')==2)
        local _, stockIcon=GetMacroInfo(2)
        assert(stockIcon=='Interface\\Icons\\Spell_Nature_Heal')
        -- The table is kept sorted by name, as 1.12.1's is, so each answer is its sorted place.
        for i=3,18 do assert(type(C_Macro.CreateMacro('Full'..i, nil, nil))=='number') end
        assert(C_Macro.CreateMacro('Overflow', nil, nil)==nil)
    "#,
        )
        .unwrap();
    assert!(script.take_macros_dirty());
    assert!(!script.take_macros_dirty());
    let macros = script.macros();
    let item = macros.saved(false).into_iter().find(|m| m.name == "Item");
    assert_eq!(item.map(|m| m.body.as_str()), Some("/bow"));
    assert_eq!(
        script.macros().saved(true)[0].texture.as_deref(),
        Some("Interface\\Icons\\INV_Potion_51")
    );
    assert_eq!(script.macros_generation(), 21);
}

#[test]
fn reputation_structs_follow_visible_indices_folds_and_watched_slot_zero() {
    use benilla_ui::script::{FactionEntry, ReputationState};
    let mut script = extended();
    let header = FactionEntry {
        faction_id: 469,
        rep_list_id: 90,
        parent_id: 0,
        name: "Alliance".into(),
        description: String::new(),
        standing: 0,
        standing_id: 4,
        bar_min: 0,
        bar_max: 3000,
        visible: true,
        is_header: true,
        at_war: false,
        can_toggle_at_war: false,
        inactive: false,
    };
    let child = FactionEntry {
        faction_id: 72,
        rep_list_id: 0,
        parent_id: 469,
        name: "Stormwind".into(),
        description: "Home".into(),
        standing: 4500,
        standing_id: 5,
        bar_min: 3000,
        bar_max: 9000,
        visible: true,
        is_header: false,
        at_war: false,
        can_toggle_at_war: true,
        inactive: false,
    };
    script.set_reputation(ReputationState {
        entries: vec![child, header],
        watched: Some(0),
    });
    script
        .run(
            r#"
        local header=C_Reputation.GetFactionDataByIndex(1)
        local faction=C_Reputation.GetFactionDataByIndex(2)
        assert(header.factionID==469 and header.isHeader and not header.isCollapsed)
        assert(faction.factionID==72 and faction.currentStanding==4500 and faction.reaction==5)
        assert(faction.currentReactionThreshold==3000 and faction.nextReactionThreshold==9000)
        assert(not faction.isHeader and faction.isWatched and faction.canSetInactive)
        assert(C_Reputation.GetWatchedFactionData().factionID==72)
        faction.name='Tampered'
        assert(C_Reputation.GetWatchedFactionData().name=='Stormwind')
        CollapseFactionHeader(1)
        assert(C_Reputation.GetFactionDataByIndex(1).isCollapsed)
        assert(C_Reputation.GetFactionDataByIndex(2)==nil)
        assert(C_Reputation.GetWatchedFactionData().factionID==72)
        assert(C_Reputation.GetFactionDataByIndex(0)==nil)
        assert(C_Reputation.GetFactionDataByIndex(-1)==nil)
        ExpandFactionHeader(1)
        SetFactionInactive(2)
    "#,
        )
        .unwrap();
    // The verb is the host's to apply, by the row's reputation slot; this VM keeps it.
    assert_eq!(
        script.take_script_calls(),
        vec![benilla_ui::script::ScriptCall::Reputation(
            benilla_ui::script::ReputationCall::Inactive {
                rep_list_id: 0,
                inactive: true
            }
        )]
    );
}

#[test]
fn quest_details_read_cached_templates_independently_of_log_selection() {
    use benilla_ui::script::pfui_host::quests::{set_details, Detail, Requirement};
    let mut script = extended();
    seat_quest_log(&mut script, &[(42, "Zone", "Bring supplies")]);
    script
        .run("assert(C_QuestLog.GetQuestDetails(42)==nil)")
        .unwrap();
    set_details(
        script.lua(),
        [Detail {
            quest_id: 42,
            title: "Bring supplies".into(),
            level: 10,
            money: -25,
            flags: 8,
            rewards: vec![(0, 0), (7, 2)],
            requirements: vec![Requirement {
                kind: "item".into(),
                id: 99,
                count: 4,
                text: String::new(),
            }],
            ..Default::default()
        }],
    );
    script.run(r#"
        assert(C_QuestLog.GetQuestIDForLogIndex(1)==0)
        assert(C_QuestLog.GetQuestIDForLogIndex(2)==42)
        assert(C_QuestLog.GetQuestIDForLogIndex(0)==nil)
        assert(C_QuestLog.GetQuestIDForLogIndex(-1)==nil)
        assert(C_QuestLog.GetQuestIDForLogIndex(3)==nil)
        local d=C_QuestLog.GetQuestDetails(42)
        assert(d.title=='Bring supplies' and d.level==10 and d.isSharable)
        assert(d.requiredMoney==25 and d.rewardMoney==0 and d.questType==nil and d.poi==nil)
        assert(#d.rewardItems==1 and d.rewardItems[1].id==7 and #d.choiceItems==0)
        assert(d.requirements[1].kind=='item' and d.requirements[1].id==99 and d.requirements[1].count==4)
        d.requirements[1].id=123
        assert(C_QuestLog.GetQuestDetails(42).requirements[1].id==99)
        assert(C_QuestLog.GetQuestDetails(-1)==nil and C_QuestLog.GetQuestDetails(999)==nil)
    "#).unwrap();
    set_details(script.lua(), []);
    script
        .run("assert(C_QuestLog.GetQuestDetails(42)==nil)")
        .unwrap();
}

#[test]
fn opt_in_is_per_vm_and_does_not_advertise_classicapi_parity() {
    let script = extended();
    assert_eq!(script.eval::<i64>("return #{1,2,3} + 7 % 3").unwrap(), 4);
    assert_eq!(
        script
            .eval::<String>("return ('Hello'):match('H(%w+)')")
            .unwrap(),
        "ello"
    );
    script
        .run("assert(CLASSIC_API_VERSION == 11510 and BenillaClassicAPICompatibility.emulated and not BenillaClassicAPICompatibility.fullClassicAPI and type(C_EquipmentSet) == 'table')")
        .unwrap();
    // Nampower's surface is its own feature on top.
    #[cfg(feature = "nampower")]
    script
        .run("assert(GetNampowerVersion() == 3 and BenillaNampowerCompatibility.emulated)")
        .unwrap();
    #[cfg(not(feature = "nampower"))]
    script
        .run("assert(GetNampowerVersion == nil and BenillaNampowerCompatibility == nil)")
        .unwrap();
    let stock = UiScript::new().unwrap();
    for source in [
        "return #{}",
        "return 7 % 3",
        "return ...",
        "return ('x'):upper()",
    ] {
        assert!(stock.run(source).is_err(), "stock accepted {source}");
    }
    stock
        .run("assert(C_Timer == nil and Mixin == nil and string.match == nil and select == nil)")
        .unwrap();
    stock.run("assert(C_FriendList == nil and C_PlayerCache == nil and GetActionInfo == nil and GetInventoryItemID == nil and INVSLOT_HEAD == nil)").unwrap();
}

#[test]
fn action_and_inventory_queries_read_live_snapshots_and_preserve_empty_results() {
    use benilla_ui::script::{ActionSlot, InvSlotView, UnitGuids};
    let mut script = extended();
    script.set_unit_guids(&UnitGuids {
        player: 3,
        target: 3,
        ..Default::default()
    });
    let mut slots = Default::default();
    // Broken equipment must remain distinguishable from equipment without durability.
    let slots: &mut benilla_ui::script::InventorySlots = &mut slots;
    slots[5] = Some(InvSlotView {
        item_id: 10,
        durability: Some((0, 40)),
        ..Default::default()
    });
    slots[11] = Some(InvSlotView {
        item_id: 11,
        durability: Some((0, 0)),
        ..Default::default()
    });
    script.set_inventory_slots(slots.clone());
    for (slot, kind, action) in [(1, 0, 6603), (2, 0x80, 6948), (3, 0x40, 1), (4, 0x40, 36)] {
        script.set_action(
            slot,
            Some(ActionSlot {
                kind,
                action,
                ..Default::default()
            }),
        );
    }
    script
        .run(
            r#"
      assert(CreateMacro('Example',1,'/say test') == 1)
      local kind,id,subtype = GetActionInfo(1)
      assert(kind=='spell' and id==6603 and subtype=='spell')
      kind,id,subtype=GetActionInfo(2); assert(kind=='item' and id==6948 and subtype==nil)
      kind,id=GetActionInfo(3); assert(kind=='macro' and id==1)
      kind,id=GetActionInfo(4); assert(kind=='item' and id==nil)
      assert(select('#', GetActionInfo(5))==0 and GetActionInfo(121)==nil)
      assert(GetInventoryItemID('player', INVSLOT_CHEST)==10)
      assert(GetInventoryItemID('target', INVSLOT_CHEST)==10)
      assert(GetInventoryItemID('unrecognized', 5)==nil)
      assert(GetInventoryItemID('player', -1)==nil and GetInventoryItemID('player', 999)==nil)
      local cur,max=GetInventoryItemDurability(INVSLOT_CHEST); assert(cur==0 and max==40)
      assert(select('#',GetInventoryItemDurability(INVSLOT_FINGER1))==0)
      assert(INVSLOT_FIRST_EQUIPPED==1 and INVSLOT_LAST_EQUIPPED==19 and INVSLOT_AMMO==0)
    "#,
        )
        .unwrap();
    script.set_action(1, None);
    script.run("assert(GetActionInfo(1)==nil)").unwrap();
}

#[test]
fn social_records_notes_and_silent_who_queries_use_real_state() {
    use benilla_ui::script::{FriendInfo, ScriptCall, SocialCall, SocialState, UnitState, WhoInfo};
    let mut script = extended();
    // The `/who` request is a call; this VM has no host, so it keeps it.
    script.keep_calls();
    script.set_realm_name("Test Realm");
    script.set_unit(
        "player",
        Some(UnitState {
            exists: true,
            name: Some("Pfuitest".into()),
            ..Default::default()
        }),
    );
    // The status tags are GlobalStrings' on the client.
    script
        .run("LOCALIZED_CLASS_NAMES_MALE={MAGE='Magier'} CHAT_FLAG_AFK='<AFK>' CHAT_FLAG_DND='<DND>'")
        .unwrap();
    script.set_social(SocialState {
        friends: vec![
            FriendInfo {
                name: Some("Alice".into()),
                online: true,
                level: 60,
                class: Some("Magier".into()),
                area: Some("City".into()),
                // Away.
                flags: 2,
                listed: true,
            },
            FriendInfo {
                name: Some("Bob".into()),
                listed: true,
                ..Default::default()
            },
        ],
        who: vec![WhoInfo {
            name: "Alice".into(),
            class: Some("Magier".into()),
            guild: "Guild".into(),
            level: 60,
            zone: Some("City".into()),
            race: Some("Mensch".into()),
        }],
        who_total: 42,
        ..Default::default()
    });
    benilla_ui::script::pfui_host::social::set_friend_guids(script.lua(), [("Alice", 0xab)])
        .unwrap();
    script.run(r#"
      assert(C_FriendList.GetNumOnlineFriends()==1 and C_FriendList.GetNumFriends()==2)
      local info=C_FriendList.GetFriendInfo('aLiCe')
      assert(info.name=='Alice' and info.classFilename=='MAGE' and info.connected)
      assert(info.guid=='0x00000000000000AB' and info.afk and not info.dnd)
      assert(C_FriendList.GetFriendInfoByIndex(0)==nil and C_FriendList.GetFriendInfo('missing')==nil)
      local f=CreateFrame('Frame'); f:RegisterEvent('FRIENDLIST_UPDATE')
      friendUpdates=0; f:SetScript('OnEvent',function() friendUpdates=friendUpdates+1 end)
      C_FriendList.SetFriendNotes('ALICE','raid healer')
      assert(C_FriendList.GetFriendInfoByIndex(1).notes=='raid healer' and friendUpdates==1)
      C_FriendList.SetFriendNotes('missing','irrelevant'); assert(friendUpdates==1)
      local shown,total=C_FriendList.GetNumWhoResults(); assert(shown==1 and total==42)
      info=C_FriendList.GetWhoInfo(1)
      assert(info.filename=='MAGE' and info.fullGuildName=='Guild' and info.raceStr=='Mensch')
      assert(not C_FriendList.IsWhoQueryPending())
      assert(not C_FriendList.SendWhoQueryByName('" injected'))
      assert(C_FriendList.SendWhoQueryByName('Alice') and C_FriendList.IsWhoQueryPending())
      assert(not C_FriendList.SendWhoQueryByName('Bob'))
    "#).unwrap();
    assert!(script.who_to_ui());
    assert_eq!(
        script.take_script_calls(),
        vec![ScriptCall::Social(SocialCall::Who(b"n-\"Alice\"".to_vec()))]
    );
    script.set_now(5.0);
    script.run("assert(not C_FriendList.IsWhoQueryPending() and C_FriendList.SendWhoQueryByName('Bob'))").unwrap();
    let saved = script.saved_variables_bytes();
    let mut reloaded = extended();
    reloaded.set_realm_name("Test Realm");
    reloaded.set_unit(
        "player",
        Some(UnitState {
            name: Some("Pfuitest".into()),
            ..Default::default()
        }),
    );
    reloaded.set_social(script_social_for_note());
    reloaded.run_chunk(&saved).unwrap();
    reloaded.run("assert(C_FriendList.GetFriendInfo('Alice').notes=='raid healer') C_FriendList.SetFriendNotes('Alice',nil) assert(C_FriendList.GetFriendInfo('Alice').notes==nil)").unwrap();
    reloaded.set_unit(
        "player",
        Some(UnitState {
            name: Some("Other".into()),
            ..Default::default()
        }),
    );
    reloaded
        .run("assert(C_FriendList.GetFriendInfo('Alice').notes==nil)")
        .unwrap();
}

fn script_social_for_note() -> benilla_ui::script::SocialState {
    benilla_ui::script::SocialState {
        friends: vec![benilla_ui::script::FriendInfo {
            name: Some("Alice".into()),
            listed: true,
            ..Default::default()
        }],
        ..Default::default()
    }
}

#[test]
fn player_cache_is_opt_in_persistent_and_evicts_recycled_names() {
    let mut script = extended();
    script.set_realm_name("Realm");
    script.run(r#"
      LOCALIZED_CLASS_NAMES_MALE={MAGE='Magier', WARRIOR='Krieger'}
      assert(not C_PlayerCache.IsEnabled())
      assert(not C_PlayerCache.RememberPlayer('0x000000AB','Alice','MAGE'))
      C_PlayerCache.SetEnabled(1)
      assert(C_PlayerCache.RememberPlayer('0x000000AB','Alice','MAGE'))
      local class,token,race,raceToken,sex,name,realm,guid=C_PlayerCache.GetPlayerInfoByName('Alice')
      assert(class=='Magier' and token=='MAGE' and race==nil and sex==2 and realm=='')
      assert(guid=='0x00000000000000AB' and name=='Alice')
      assert(C_PlayerCache.GetPlayerInfoByName('alice')==nil)
      assert(C_PlayerCache.RememberPlayer('0x000000AB','Alice','UNKNOWN'))
      assert(select(2,C_PlayerCache.GetPlayerInfoByName('Alice'))=='MAGE')
      assert(not C_PlayerCache.RememberPlayer('0x0','Alice','MAGE'))
      assert(not C_PlayerCache.RememberPlayer('0x00000000','Alice','MAGE'))
      assert(C_PlayerCache.RememberPlayer('0x000000CD','Alice','WARRIOR'))
      assert(C_PlayerCache.RememberPlayer('0x000000AB','Someone','MAGE'))
      assert(select(8,C_PlayerCache.GetPlayerInfoByName('Alice'))=='0x00000000000000CD')
      C_PlayerCache.SetEnabled(0); assert(C_PlayerCache.GetPlayerInfoByName('Alice')==nil)
      C_PlayerCache.SetEnabled(true)
    "#).unwrap();
    let saved = script.saved_variables_bytes();
    let mut reloaded = extended();
    reloaded.set_realm_name("Realm");
    reloaded.run_chunk(&saved).unwrap();
    reloaded.run("assert(C_PlayerCache.IsEnabled() and select(2,C_PlayerCache.GetPlayerInfoByName('Alice'))=='WARRIOR')").unwrap();
    reloaded.set_realm_name("Other Realm");
    reloaded
        .run("assert(C_PlayerCache.GetPlayerInfoByName('Alice')==nil)")
        .unwrap();
}

#[test]
fn real_name_answers_populate_opt_in_cache_and_correct_wire_gender() {
    use super::{creature, player_cache};
    let mut script = extended();
    script.set_realm_name("Realm");
    creature::Catalog {
        classes: Default::default(),
        class_order: vec!["WARRIOR".into(), "MAGE".into()],
        races: [(
            1,
            creature::Race {
                id: 1,
                name: "Mensch".into(),
                file: "Human".into(),
                faction: None,
            },
        )]
        .into(),
    }
    .install(script.lua())
    .unwrap();
    script
        .run("LOCALIZED_CLASS_NAMES_MALE={MAGE='Magier', WARRIOR='Krieger'}")
        .unwrap();
    assert!(!player_cache::observe_name(script.lua(), 7, "Alice", Some("MAGE"), 1, 1).unwrap());
    script.run("C_PlayerCache.SetEnabled(true)").unwrap();
    assert!(player_cache::observe_name(script.lua(), 7, "Alice", Some("MAGE"), 1, 1).unwrap());
    script.run(r#"
      local class,token,race,file,sex,name,realm,guid=C_PlayerCache.GetPlayerInfoByName('Alice')
      assert(class=='Magier' and token=='MAGE' and race=='Mensch' and file=='Human' and sex==3 and name=='Alice' and realm=='' and guid=='0x0000000000000007')
      C_PlayerCache.RememberPlayer(guid,name,token,file,0)
      assert(select(5,C_PlayerCache.GetPlayerInfoByName('Alice'))==3)
    "#).unwrap();
    player_cache::observe_name(script.lua(), 7, "Alice", None, 0, 0).unwrap();
    script.run("assert(select(5,C_PlayerCache.GetPlayerInfoByName('Alice'))==2 and select(2,C_PlayerCache.GetPlayerInfoByName('Alice'))=='MAGE')").unwrap();
    player_cache::observe_name(script.lua(), 7, "Betta", Some("UNKNOWN"), 255, 255).unwrap();
    script.run("assert(C_PlayerCache.GetPlayerInfoByName('Alice')==nil and select(2,C_PlayerCache.GetPlayerInfoByName('Betta'))=='MAGE' and select(5,C_PlayerCache.GetPlayerInfoByName('Betta'))==2)").unwrap();
    player_cache::observe_name(script.lua(), 8, "Betta", Some("WARRIOR"), 1, 1).unwrap();
    script.run("C_PlayerCache.SetEnabled(false)").unwrap();
    assert!(!player_cache::observe_name(script.lua(), 8, "Betta", Some("MAGE"), 1, 0).unwrap());
    script.run("C_PlayerCache.SetEnabled(true); assert(select(2,C_PlayerCache.GetPlayerInfoByName('Betta'))=='WARRIOR' and select(5,C_PlayerCache.GetPlayerInfoByName('Betta'))==3)").unwrap();
    assert!(!player_cache::observe_name(
        script.lua(),
        0xf130000001000001,
        "NPC",
        Some("MAGE"),
        1,
        0
    )
    .unwrap());
    assert!(!player_cache::observe_name(script.lua(), 0, "Invalid", Some("MAGE"), 1, 0).unwrap());
    assert!(!player_cache::observe_name(script.lua(), 8, "", None, 0, 0).unwrap());
    script.set_realm_name("Other");
    script
        .run("assert(C_PlayerCache.GetPlayerInfoByName('Betta')==nil)")
        .unwrap();
    player_cache::observe_name(script.lua(), 8, "Betta", Some("MAGE"), 1, 0).unwrap();
    script.set_realm_name("Realm");
    script
        .run("assert(select(2,C_PlayerCache.GetPlayerInfoByName('Betta'))=='WARRIOR')")
        .unwrap();
    let stock = UiScript::new().unwrap();
    assert!(!player_cache::observe_name(stock.lua(), 8, "Betta", Some("MAGE"), 1, 0).unwrap());
}

#[test]
fn late_loot_identity_updates_finished_rolls_once_without_changing_results() {
    use super::loot::{self, Event, Key, Player};
    let script = extended();
    script.run(r#"
      identityChanges=0; completions=0
      local f=CreateFrame('Frame'); f:RegisterEvent('LOOT_HISTORY_ROLL_CHANGED'); f:RegisterEvent('LOOT_HISTORY_ROLL_COMPLETE')
      f:SetScript('OnEvent',function(self,event,item,player)
        assert(item==1)
        if event=='LOOT_HISTORY_ROLL_CHANGED' then assert(player==1); identityChanges=identityChanges+1
        else completions=completions+1 end
      end)
    "#).unwrap();
    loot::observe(
        script.lua(),
        Key {
            target: 99,
            slot: 1,
        },
        123,
        0,
        10.,
        Event::Won(Player {
            guid: 7,
            name: None,
            class: None,
            kind: 1,
            roll: 88,
            is_me: false,
        }),
    );
    script.run("assert(C_LootHistory.GetPlayerInfo(1,1)==nil and identityChanges==1 and completions==1)").unwrap();
    loot::resolve_player(script.lua(), 8, "Someone", Some("MAGE"));
    loot::resolve_player(script.lua(), 7, "", Some("MAGE"));
    loot::resolve_player(script.lua(), 7, "Alice", Some("MAGE"));
    loot::resolve_player(script.lua(), 7, "Renamed", Some("WARRIOR"));
    script
        .run(
            r#"
      local name,class,kind,roll,winner,me=C_LootHistory.GetPlayerInfo(1,1)
      assert(name=='Alice' and class=='MAGE' and kind==1 and roll==88 and winner and not me)
      local id,link,count,done,winningIndex,time=C_LootHistory.GetItem(1)
      assert(id==1 and count==1 and done and winningIndex==1 and time==10)
      assert(identityChanges==2 and completions==1)
    "#,
        )
        .unwrap();
}

#[test]
fn mount_stealth_and_form_helpers_follow_live_descriptors_and_authorized_dbc_auras() {
    use super::spell::{Catalog, Spell};
    use benilla_ui::script::{
        pfui_host::units::{self, RawUnit},
        AuraState, UnitGuids, UnitState,
    };
    use std::collections::BTreeMap;
    let mut script = extended();
    let spell = |aura, form| Spell {
        name: "State".into(),
        rank: String::new(),
        icon: None,
        dispel: 0,
        attributes: 0,
        reagents: vec![],
        effects: vec![BTreeMap::from([
            ("effect", 6.0),
            ("auraName", aura),
            ("miscValue", form),
        ])],
        cast_time: 0,
        min_range: 0.0,
        max_range: 0.0,
        cost: 0,
        power_type: 0,
        funnel: false,
        swing: None,
    };
    Catalog(BTreeMap::from([
        (100, spell(78.0, 0.0)),
        (101, spell(16.0, 0.0)),
        (102, spell(36.0, 28.0)),
        (103, spell(36.0, 1.0)),
    ]))
    .install(script.lua())
    .unwrap();
    script.run("assert(not IsMounted() and not IsStealthed() and GetShapeshiftFormID()==0); Dismount(); CancelShapeshiftForm()").unwrap();
    script.set_unit_guids(&UnitGuids {
        player: 3,
        ..Default::default()
    });
    script.set_unit(
        "player",
        Some(UnitState {
            class_file: Some("PRIEST".into()),
            ..Default::default()
        }),
    );
    script.set_player_auras(
        [100, 101, 102, 103]
            .into_iter()
            .map(|spell_id| AuraState {
                spell_id,
                helpful: true,
                cancelable: true,
                ..Default::default()
            })
            .collect(),
    );
    let feed = |form, mount_display, auras| {
        units::set_raw_units(
            script.lua(),
            [(
                3,
                RawUnit {
                    form,
                    mount_display,
                    auras,
                    health: Some(10),
                    applications: vec![],
                },
            )],
        )
    };
    feed(28, 123, vec![100, 101, 103, 102]);
    script
        .run("assert(IsMounted() and not IsStealthed()); Dismount(); CancelShapeshiftForm()")
        .unwrap();
    assert_eq!(script.take_cancel_aura_requests(), [100, 102]);
    units::set_raw_units(
        script.lua(),
        [(
            3,
            RawUnit {
                form: 28,
                mount_display: 0,
                auras: vec![101, 103, 102],
                health: Some(10),
                applications: vec![],
            },
        )],
    );
    script
        .run("assert(not IsMounted() and IsStealthed()); Dismount(); CancelShapeshiftForm()")
        .unwrap();
    assert_eq!(script.take_cancel_aura_requests(), [102]);
    script.set_unit(
        "player",
        Some(UnitState {
            class_file: Some("WARRIOR".into()),
            ..Default::default()
        }),
    );
    script.run("CancelShapeshiftForm()").unwrap();
    assert!(script.take_cancel_aura_requests().is_empty());
    script.set_unit(
        "player",
        Some(UnitState {
            class_file: Some("PRIEST".into()),
            ..Default::default()
        }),
    );
    script.set_player_auras(vec![AuraState {
        spell_id: 102,
        helpful: true,
        cancelable: false,
        ..Default::default()
    }]);
    script.run("CancelShapeshiftForm()").unwrap();
    assert!(script.take_cancel_aura_requests().is_empty());
    let mut auras = vec![0; 33];
    auras[32] = 101;
    units::set_raw_units(
        script.lua(),
        [(
            3,
            RawUnit {
                form: 0,
                mount_display: 0,
                auras,
                health: Some(10),
                applications: vec![],
            },
        )],
    );
    script
        .run("assert(not IsStealthed()); CancelShapeshiftForm()")
        .unwrap();
    assert!(script.take_cancel_aura_requests().is_empty());
}

#[test]
fn shapeshift_id_reads_player_descriptor_instead_of_stance_bar_index() {
    use benilla_ui::script::{
        pfui_host::units::{self, RawUnit},
        UnitGuids,
    };
    let mut script = extended();
    script.run("assert(GetShapeshiftFormID()==0)").unwrap();
    script.set_unit_guids(&UnitGuids {
        player: 3,
        target: 4,
        ..Default::default()
    });
    for form in [0, 1, 17, 28] {
        units::set_raw_units(
            script.lua(),
            [
                (
                    3,
                    RawUnit {
                        mount_display: 0,
                        form,
                        health: Some(10),
                        auras: vec![],
                        applications: vec![],
                    },
                ),
                (
                    4,
                    RawUnit {
                        mount_display: 0,
                        form: 2,
                        health: Some(10),
                        auras: vec![],
                        applications: vec![],
                    },
                ),
            ],
        );
        assert_eq!(
            script
                .eval::<Option<u8>>("return GetShapeshiftFormID()")
                .unwrap(),
            Some(form)
        );
    }
    units::set_raw_units(script.lua(), []);
    script.run("assert(GetShapeshiftFormID()==0)").unwrap();
    let stock = UiScript::new().unwrap();
    stock.run("assert(GetShapeshiftFormID==nil)").unwrap();
}

#[test]
fn frame_spell_and_macro_actions_capture_guid_without_changing_selection() {
    use benilla_ui::script::{
        pfui_host::attributes::ClickAction, ScriptCall, SpellBookState, SpellSlotView, UnitGuids,
    };
    let mut script = extended();
    script.set_unit_guids(&UnitGuids {
        player: 3,
        target: 5,
        party: [4, 0, 0, 0],
        ..Default::default()
    });
    script.set_spellbook(SpellBookState {
        slots: vec![
            SpellSlotView {
                spell_id: 100,
                name: "Heal".into(),
                rank: Some("Rank 1".into()),
                ..Default::default()
            },
            SpellSlotView {
                spell_id: 101,
                name: "Heal".into(),
                rank: Some("Rank 2".into()),
                ..Default::default()
            },
        ],
        ..Default::default()
    });
    script.run(r#"
      clickFrame=CreateFrame('Button'); local f=clickFrame
      clicks=0; f:SetScript('OnClick',function() clicks=clicks+1 end)
      f:SetAttribute('unit','party1'); f:SetAttribute('type1','spell'); f:SetAttribute('spell1','Heal')
      f:Click('LeftButton'); assert(clicks==0 and UnitGUID('target')=='0x0000000000000005')
      f:SetAttribute('spell1','Heal(Rank 1)'); f:Click('LeftButton')
      f:SetAttribute('spell1','Unknown'); f:Click('LeftButton'); assert(clicks==0)
      f:SetAttribute('unit','party2'); f:SetAttribute('spell1',100); f:Click('LeftButton')
      f:ClearAttribute('unit'); f:SetAttribute('spell1','100'); f:Click('LeftButton')
      f:SetAttribute('type1','macro'); f:SetAttribute('macro1','Named'); f:Click('LeftButton')
      f:SetAttribute('macrotext1','/cast Heal'); f:SetAttribute('unit','party1'); f:Click('LeftButton')
      assert(clicks==0 and UnitGUID('target')=='0x0000000000000005')
    "#).unwrap();
    assert_eq!(
        script.take_script_calls(),
        vec![
            ScriptCall::PfuiClick(ClickAction::Spell {
                spell_id: 101,
                target: Some(4)
            }),
            ScriptCall::PfuiClick(ClickAction::Spell {
                spell_id: 100,
                target: Some(4)
            }),
            ScriptCall::PfuiClick(ClickAction::Spell {
                spell_id: 100,
                target: None
            }),
            ScriptCall::PfuiClick(ClickAction::Macro {
                input: "Named".into(),
                named: true,
                target: None
            }),
            ScriptCall::PfuiClick(ClickAction::Macro {
                input: "/cast Heal".into(),
                named: false,
                target: Some(4)
            }),
        ]
    );
}

#[test]
fn click_macros_use_real_bodies_and_scope_lua_target_reads_even_on_error() {
    use benilla_ui::script::{
        pfui_host::{attributes, units},
        MacroState, MacroView, UnitGuids, UnitState,
    };
    let mut script = extended();
    script.set_unit_guids(&UnitGuids {
        player: 3,
        target: 5,
        held: [(3, 0), (5, 0)].into(),
        ..Default::default()
    });
    let state = |guid, name: &str| UnitState {
        guid,
        name: Some(name.into()),
        exists: true,
        has_object: true,
        ..Default::default()
    };
    script.set_unit("target", Some(state(5, "Original")));
    units::set_unit_states(
        script.lua(),
        [(4, state(4, "Clicked"), 3), (3, state(3, "Player"), 0)],
    );
    script.set_macros(MacroState::from_scopes(
        vec![MacroView {
            name: "Named".into(),
            body: " /cast Heal\r\n\n/script second() ".into(),
            ..Default::default()
        }],
        Vec::new(),
    ));
    script
        .run(
            r#"
      lines={}; local f=CreateFrame('Frame'); f:RegisterEvent('EXECUTE_CHAT_LINE')
      f:SetScript('OnEvent',function(self,evt,line)
        assert(evt=='EXECUTE_CHAT_LINE' and arg1==line)
        assert(UnitName('target')=='Clicked' and UnitName('targettarget')=='Player')
        table.insert(lines,line)
      end)
    "#,
        )
        .unwrap();
    attributes::run_macro(script.lua(), "Named", true, Some(4)).unwrap();
    script.run("assert(#lines==2 and lines[1]=='/cast Heal' and lines[2]=='/script second()'); assert(UnitName('target')=='Original' and UnitGUID('targettarget')==nil)").unwrap();
    attributes::run_macro(script.lua(), "1", true, Some(4)).unwrap();
    script.run("assert(#lines==4)").unwrap();
    attributes::run_macro(script.lua(), "Missing", true, Some(4)).unwrap();
    script
        .run(
            r#"
      assert(#lines==4)
      function RunMacro(input)
        assert(input=='addon syntax' and UnitName('target')=='Clicked')
        addonRan=true; error('macro failure')
      end
    "#,
        )
        .unwrap();
    assert!(attributes::run_macro(script.lua(), "addon syntax", false, Some(4)).is_err());
    script.run("assert(addonRan and UnitName('target')=='Original' and UnitGUID('target')=='0x0000000000000005' and #lines==4)").unwrap();
    script
        .run("RunMacro=function() assert(UnitGUID('target')==nil and UnitName('target')==nil) end")
        .unwrap();
    attributes::run_macro(script.lua(), "empty selection", false, None).unwrap();
    script
        .run("assert(UnitName('target')=='Original')")
        .unwrap();
}

#[test]
fn frame_attributes_deliver_handlers_and_drive_one_real_action_per_click() {
    use benilla_ui::script::{ScriptCall, SelectionRequest};
    let mut script = extended();
    script.run(r#"
      attrButton=CreateFrame('Button')
      local frame=attrButton
      attrChanges=0; customClicks=0
      frame:SetScript('OnAttributeChanged',function(self,key,value)
        assert(self==frame and this==frame and key==arg1 and value==arg2)
        attrChanges=attrChanges+1
        -- Recursive changes update values but do not reenter the callback.
        self:SetAttribute('nested',value)
      end)
      assert(frame:HasScript('OnAttributeChanged'))
      frame:SetAttribute('UNIT','party1')
      assert(attrChanges==1 and frame:GetAttribute('unit')=='party1' and frame:GetAttribute('nested')=='party1')
      frame:SetAttributeNoHandler('type1','target')
      frame:SetAttributeNoHandler('shift-type1','assist')
      frame:SetAttributeNoHandler('*test*',false)
      assert(frame:GetAttribute('shift-','test','1')==false)
      assert(attrChanges==1 and frame:ClearAttribute('test')==false)
      frame:SetScript('OnClick',function() customClicks=customClicks+1 end)
      frame:Click('LeftButton')
      assert(customClicks==0)
      assert(frame:ClearAttribute('type1') and not frame:ClearAttribute('type1'))
      frame:Click('LeftButton'); assert(customClicks==1)
      frame:SetAttribute('type1','target')
      frame:Click('LeftButton'); assert(customClicks==1)
    "#).unwrap();
    assert_eq!(
        script.take_script_calls(),
        vec![
            ScriptCall::Select {
                request: SelectionRequest::Unit("party1".into()),
                may_cast: true
            },
            ScriptCall::Select {
                request: SelectionRequest::Unit("party1".into()),
                may_cast: true
            },
        ]
    );
    script.set_modifiers(true, false, false);
    script.run("attrButton:Click('LeftButton')").unwrap();
    assert_eq!(
        script.take_script_calls(),
        vec![ScriptCall::Select {
            request: SelectionRequest::Assist("party1".into()),
            may_cast: true
        }]
    );
    script.run(r#"
      this='outer'; arg1='original'; arg2='second'
      attrButton:SetScript('OnAttributeChanged',function() error('attribute callback failed') end)
      assert(not pcall(function() attrButton:SetAttribute('unit','player') end))
      assert(attrButton:GetAttribute('unit')=='player' and this=='outer' and arg1=='original' and arg2=='second')
      attrButton:SetScript('OnAttributeChanged',nil)
      attrButton:SetAttributeNoHandler('unit','none')
      attrButton:ClearAttribute('shift-type1')
      attrButton:Click('LeftButton')
    "#).unwrap();
    assert_eq!(script.take_script_calls(), vec![ScriptCall::ClearTarget]);
    let stock = UiScript::new().unwrap();
    assert!(stock
        .run("CreateFrame('Button'):SetAttribute('unit','player')")
        .is_err());
}

#[test]
fn modern_auras_keep_descriptor_slots_and_report_only_known_casters_and_timers() {
    use benilla_ui::script::{
        pfui_host::auras::{self, Aura},
        AuraState, UnitGuids,
    };
    let mut script = extended();
    script.set_unit_guids(&UnitGuids {
        player: 3,
        pet: 4,
        target: 5,
        ..Default::default()
    });
    let aura = |slot, spell, helpful, source_guid| Aura {
        crowd_control: false,
        self_only: false,
        applies_aura: false,
        immunities: Vec::new(),
        slot,
        source_guid,
        state: AuraState {
            spell_id: spell,
            name: Some(format!("Aura{spell}")),
            icon: Some("Icon".into()),
            count: 2,
            helpful,
            debuff_type: (!helpful).then(|| "Magic".into()),
            ..Default::default()
        },
    };
    auras::set_auras(
        script.lua(),
        5,
        Some(vec![
            aura(31, 101, true, None),
            aura(0, 100, true, None),
            aura(5, 103, false, Some(4)),
            aura(33, 102, false, Some(3)),
        ]),
    );
    script.run(r#"
      local token,first,second=C_UnitAuras.GetAuraSlots('target','HELPFUL')
      assert(token==nil and first==0 and second==31)
      local info=C_UnitAuras.GetAuraDataByIndex('target',1,'HELPFUL')
      assert(info.spellId==100 and info.sourceGUID==nil and info.sourceUnit==nil)
      assert(info.duration==0 and info.expirationTime==0 and not info.isFromPlayerOrPlayerPet)
      info=C_UnitAuras.GetAuraDataBySlot('0x00000005',33)
      assert(info.spellId==102 and info.sourceUnit=='player' and info.sourceGUID=='0x0000000000000003')
      assert(info.isHarmful and info.isFromPlayerOrPlayerPet and info.dispelName=='Magic')
      token,first=C_UnitAuras.GetAuraSlots('target','HARMFUL|PLAYER',1)
      assert(token~=nil and first==33)
      local nextToken,nextSlot=C_UnitAuras.GetAuraSlots('target','HARMFUL|PLAYER',1,token)
      assert(nextToken==nil and nextSlot==5)
      local output={9,8,7,6,n=4}
      local continuation,count=C_UnitAuras.GetAuraSlots('target','HELPFUL',nil,nil,output)
      assert(continuation==nil and count==2 and output.n==2 and output[1]==0 and output[2]==31)
      assert(output[3]==nil and output[4]==nil)
      continuation,count=C_UnitAuras.GetAuraSlots('target','HARMFUL|!DISPELLABLE',nil,nil,output)
      assert(count==0 and output.n==0 and output[1]==nil)
      assert(C_UnitAuras.GetAuraDataByIndex('target',1,'HELPFUL|PLAYER')==nil)
      assert(C_UnitAuras.GetAuraDataByIndex('target',0)==nil)
      assert(C_UnitAuras.GetAuraDataBySlot('target',2)==nil)
      assert(C_UnitAuras.GetAuraDataBySlot('target',nil)==nil)
      assert(C_UnitAuras.UnitAuraBySlot('target',nil)==nil)
      local name,icon,count,dispel,duration,expiration,source,stealable,personal,spell=
        C_UnitAuras.UnitAuraBySlot('target',33)
      assert(name=='Aura102' and icon=='Icon' and count==2 and source=='player' and spell==102)
      assert(select('#',C_UnitAuras.UnitAuraBySlot('target',33))==16)
      assert(select(16,C_UnitAuras.UnitAuraBySlot('target',33))==info.auraInstanceID)
      assert(C_UnitAuras.GetAuraDataBySpellName('target','Aura102').spellId==102)
      local r,g,b,a=C_UnitAuras.GetAuraDispelTypeColor('Magic'):GetRGBA()
      assert(r==0.2 and g==0.6 and b==1 and a==1)
    "#).unwrap();
    // Removing an earlier slot does not renumber a still-occupied slot.
    auras::set_auras(script.lua(), 5, Some(vec![aura(31, 101, true, None)]));
    script.run("local _,slot=C_UnitAuras.GetAuraSlots('target'); assert(slot==31 and C_UnitAuras.GetAuraDataBySlot('target',0)==nil)").unwrap();
    let mut timed = aura(7, 200, true, None);
    timed.state.duration = 40.0;
    timed.state.expiration_time = 90.0;
    auras::set_auras(script.lua(), 3, Some(vec![timed]));
    script.set_now(55.0);
    script.run("local info=C_UnitAuras.GetAuraDataByIndex('player',1); assert(info.duration==40 and info.expirationTime==90)").unwrap();
    auras::set_auras(script.lua(), 5, None);
    script
        .run("assert(C_UnitAuras.GetAuraDataBySlot('target',31)==nil)")
        .unwrap();
}

#[test]
fn modern_aura_cancel_reuses_the_real_player_cancel_authorization() {
    use benilla_ui::script::{pfui_host::auras, AuraState};
    let mut script = extended();
    script.run("C_Spell={}").unwrap();
    auras::install_spell_cancel(script.lua()).unwrap();
    script.set_player_auras(vec![
        AuraState {
            spell_id: 1,
            helpful: true,
            cancelable: true,
            ..Default::default()
        },
        AuraState {
            spell_id: 2,
            helpful: true,
            cancelable: false,
            ..Default::default()
        },
        AuraState {
            spell_id: 3,
            helpful: false,
            cancelable: true,
            ..Default::default()
        },
        AuraState {
            spell_id: 4,
            helpful: false,
            channeled: true,
            ..Default::default()
        },
    ]);
    script
        .run("for id=1,5 do C_Spell.CancelSpellByID(id) end")
        .unwrap();
    assert_eq!(script.take_cancel_aura_requests(), vec![1, 4]);
}

#[test]
fn observed_cast_queries_and_events_share_ids_timing_and_unit_filters() {
    use benilla_ui::script::{
        pfui_host::casts::{self, Cast},
        UnitGuids,
    };
    let mut script = extended();
    script.run("C_Spell={}").unwrap();
    casts::install(script.lua()).unwrap();
    script.set_unit_guids(&UnitGuids {
        player: 3,
        target: 3,
        ..Default::default()
    });
    let cast = Cast {
        spell_id: 100,
        name: "Cast".into(),
        rank: "Rank 2".into(),
        icon: Some("Icon".into()),
        start_ms: 1000.0,
        end_ms: 3000.0,
        cast_guid: "Cast-3-1".into(),
        tradeskill: false,
        delay_ms: 200,
        channel: false,
        silenceable: false,
    };
    casts::set_casts(script.lua(), [(3, cast.clone())]);
    script.run(r#"
      local name,display,icon,start,finish,trade,id,immune,spell,bar,delay=C_Spell.UnitCastingInfo('target')
      assert(name=='Cast' and display==name and icon=='Icon' and start==1000 and finish==3000)
      assert(not trade and id=='Cast-3-1' and immune==false and spell==100 and bar==nil and delay==200)
      assert(select('#',C_Spell.UnitCastingInfo('player'))==11)
      assert(C_Spell.UnitChannelInfo('player')==nil and C_Spell.UnitCastingInfo('pet')==nil)
      castEvents=0
      local frame=CreateFrame('Frame'); frame:RegisterUnitEvent('UNIT_SPELLCAST_START','player')
      frame:SetScript('OnEvent',function(self,eventName,unit,guid,spell,name,rank)
        assert(unit=='player' and arg1==unit and guid=='Cast-3-1' and spell==100 and name=='Cast' and rank=='Rank 2')
        assert(eventName==event and this==self and arg5==rank)
        castEvents=castEvents+1
      end)
    "#).unwrap();
    casts::event(script.lua(), 3, "UNIT_SPELLCAST_START", &cast);
    script.run("assert(castEvents==1)").unwrap();
    casts::set_casts(
        script.lua(),
        [(
            3,
            Cast {
                channel: true,
                ..cast
            },
        )],
    );
    script.run(r#"
      assert(C_Spell.UnitCastingInfo('player')==nil)
      local name,_,_,start,finish,trade,immune,spell=C_Spell.ChannelInfo()
      assert(name=='Cast' and start==1000 and finish==3000 and spell==100 and immune==false and not trade)
      assert(select('#',C_Spell.UnitChannelInfo('0x00000003'))==8)
    "#).unwrap();
    casts::set_casts(script.lua(), []);
    script.run("assert(C_Spell.ChannelInfo()==nil)").unwrap();
}

#[test]
fn modern_varargs_preserve_nil_arguments_and_old_framexml_varargs() {
    let script = extended();
    script
        .run(
            r#"
      function modern(...) return select('#', ...), select(-1, ...) end
      local count, last = modern(nil, 'x', nil, 'last')
      assert(count == 4 and last == 'last')
      function legacy(...) return arg.n, arg[3] end
      local n, value = legacy('a', nil, 'c')
      assert(n == 3 and value == 'c')
      local packed = SafePack('name', nil, 'icon', nil)
      assert(packed.n == 4 and packed[1] == 'name' and packed[2] == nil and packed[3] == 'icon')
      assert(select('#',unpack(packed)) == 4 and select(3,unpack(packed)) == 'icon')
      assert(SafePack().n == 0 and SafePack(nil,nil).n == 2)
      assert(select('#', select(2, 'a', nil, 'c')) == 2)
      assert(select(2, 'a', nil, 'c') == nil)
      assert(not pcall(select, 0, 'x'))
      assert(not pcall(select, -2, 'x'))
      assert(select('#', select(5, 'x')) == 0)
      assert(select('2.0', 'a', 'b') == 'b')
      assert(string.format('%2$s %1$s', 'world', 'hello') == 'hello world')
      assert(math.mod(7, 3) == 1)
      assert(io == nil and os == nil and package == nil and debug == nil)
    "#,
        )
        .unwrap();
}

#[test]
fn profile_transport_decodes_standard_wire_fixtures_and_preserves_config() {
    let script = extended();
    script
        .run(
            r#"
      local api = C_EncodingUtil
      assert(api.EncodeBase64('hello') == 'aGVsbG8=')
      assert(api.DecodeBase64('AP8=') == string.char(0,255))
      -- CBOR RFC 8949 examples: [1,2,3], {"a":1}, and a byte string.
      local array = api.DeserializeCBOR(string.char(131,1,2,3))
      assert(array[1] == 1 and array[3] == 3)
      local map = api.DeserializeCBOR(string.char(161,97,97,1))
      assert(map.a == 1)
      assert(api.DeserializeCBOR(string.char(66,0,255)) == string.char(0,255))
      local config = {global={font_size='12'}, disabled={bags='1'}, layout={1,2,3}, enabled=true}
      local wire = api.EncodeBase64(api.CompressString(api.SerializeCBOR(config)))
      local restored = api.DeserializeCBOR(api.DecompressString(api.DecodeBase64(wire)))
      assert(restored.global.font_size == '12' and restored.disabled.bags == '1')
      assert(restored.layout[3] == 3 and restored.enabled == true)
      local json = api.DeserializeJSON('{"value":12,"array":[true,false],"text":"a\\nb"}')
      assert(json.value == 12 and json.array[2] == false and json.text == 'a\nb')
      assert(api.DeserializeJSON(api.SerializeJSON(config)).global.font_size == '12')
      local mixed = {[5]='five', ['5']='string five'}
      local restoredMixed = api.DeserializeCBOR(api.SerializeCBOR(mixed))
      assert(restoredMixed[5] == 'five' and restoredMixed['5'] == 'string five')
    "#,
        )
        .unwrap();
}

#[test]
fn profile_import_rejects_malformed_data_cycles_and_executable_values() {
    let script = extended();
    script
        .run(
            r#"
      local api = C_EncodingUtil
      assert(not pcall(api.DecodeBase64, '??'))
      assert(not pcall(api.DecompressString, 'not zlib'))
      assert(not pcall(api.DeserializeJSON, 'return {value=1}'))
      assert(not pcall(api.DeserializeCBOR, string.char(1,2)))
      assert(not pcall(api.DeserializeCBOR, string.char(131,1)))
      assert(not pcall(api.SerializeCBOR, {callback=function() end}))
      local cycle = {}; cycle.self = cycle
      assert(not pcall(api.SerializeCBOR, cycle))
      assert(not pcall(api.SerializeJSON, {[3]='hole'}))
    "#,
        )
        .unwrap();
}

#[test]
fn addon_files_share_a_private_table_without_changing_the_stock_loader() {
    let script = extended();
    let private = script.lua().create_table().unwrap();
    super::run_addon_chunk(
        script.lua(),
        b"local name, private = ...; private.name = name; private.count = 1",
        "@Interface\\AddOns\\pfUI\\one.lua",
        "pfUI",
        private.clone(),
    )
    .unwrap();
    super::run_addon_chunk(script.lua(), b"local name, private = ...; assert(private.name == name); private.count = private.count + 1",
        "@Interface\\AddOns\\pfUI\\two.lua", "pfUI", private.clone()).unwrap();
    assert_eq!(private.get::<u32>("count").unwrap(), 2);
    script.run("assert(select('#', ...) == 0)").unwrap();
}

#[test]
fn installation_is_idempotent_and_refuses_to_replace_another_extension() {
    let mut script = extended();
    script
        .run("timerCount = 0; C_Timer.After(0, function() timerCount = timerCount + 1 end)")
        .unwrap();
    super::install(&script).unwrap();
    script.tick(0.1);
    assert_eq!(script.eval::<u32>("return timerCount").unwrap(), 1);
    let collision = UiScript::new().unwrap();
    collision
        .run("C_Timer = {owner = 'another extension'}")
        .unwrap();
    assert!(super::install(&collision).is_err());
    collision
        .run("assert(Mixin == nil and C_Timer.owner == 'another extension')")
        .unwrap();
    assert!(collision.run("return #{}").is_err());
}

#[test]
fn timer_errors_do_not_poison_the_queue_and_nested_timers_wait_a_frame() {
    let mut script = extended();
    script
        .run(
            r#"
      calls = ''
      C_Timer.After(0, function() error('broken timer') end)
      C_Timer.After(0, function()
        calls = calls .. 'a'
        C_Timer.After(0, function() calls = calls .. 'c' end)
      end)
      C_Timer.After(0, function() calls = calls .. 'b' end)
    "#,
        )
        .unwrap();
    script.tick(0.1);
    assert_eq!(script.eval::<String>("return calls").unwrap(), "ab");
    script.tick(0.1);
    assert_eq!(script.eval::<String>("return calls").unwrap(), "abc");
    assert_eq!(script.errors().len(), 1);
    assert!(script.errors()[0].contains("broken timer"));
}

#[test]
fn timers_cancel_expire_and_repeat_without_a_catchup_loop() {
    let mut script = extended();
    script
        .run(
            r#"
      calls = 0
      cancelled = C_Timer.NewTimer(0, function() calls = calls + 100 end)
      cancelled:Cancel()
      cancelled:Cancel()
      assert(cancelled:IsCancelled())
      ticker = C_Timer.NewTicker(0.1, function(self)
        assert(self == ticker)
        calls = calls + 1
      end, 2)
      timer = C_Timer.NewTimer(0.5, function(self) assert(self == timer) end)
      selfCancel = C_Timer.NewTicker(0, function(self) self:Cancel() end)
    "#,
        )
        .unwrap();
    script.tick(0.25);
    script
        .run("assert(calls == 1 and not ticker:IsCancelled() and not timer:IsCancelled())")
        .unwrap();
    script.tick(0.25);
    script.run("assert(calls == 2 and ticker:IsCancelled() and timer:IsCancelled() and selfCancel:IsCancelled())").unwrap();
    script.tick(1.0);
    assert_eq!(script.eval::<u32>("return calls").unwrap(), 2);
    assert!(script.errors().is_empty(), "{:?}", script.errors());
}

#[test]
fn one_callback_can_cancel_another_due_callback() {
    let mut script = extended();
    script
        .run(
            r#"
      called = false
      C_Timer.After(0, function() second:Cancel() end)
      second = C_Timer.NewTimer(0, function() called = true end)
    "#,
        )
        .unwrap();
    script.tick(0.1);
    script.run("assert(not called)").unwrap();
}

#[test]
fn timers_fire_after_the_frames_on_update_walk_and_not_under_the_loading_screen() {
    let mut script = extended();
    script
        .run(
            r#"
      order = ''
      local f = CreateFrame('Frame')
      f:SetScript('OnUpdate', function() order = order .. 'u' end)
      C_Timer.After(0, function() order = order .. 't' end)
    "#,
        )
        .unwrap();
    script.tick_covered(0.1);
    assert_eq!(script.eval::<String>("return order").unwrap(), "");
    script.tick(0.1);
    assert_eq!(script.eval::<String>("return order").unwrap(), "ut");
    assert!(script.errors().is_empty(), "{:?}", script.errors());
}

#[test]
fn mixins_and_colors_support_pfuis_class_colors_and_tooltips() {
    let script = extended();
    script
        .run(
            r#"
      local color = Mixin({r=1, g=0.2, b=0, a=1}, ColorMixin)
      assert(color:GenerateHexColor() == 'ffff3300')
      assert(color:GenerateHexColorMarkup() == '|cffff3300')
      assert(color:WrapTextInColorCode('mage') == '|cffff3300mage|r')
      assert(C_ColorUtil.GenerateTextColorCode(color) == 'ffff3300')
      assert(C_ColorUtil.GenerateTextColorCode({r=1,g=0,b=0}) == 'ffff0000')
      local r,g,b,a = CreateColor(0,1,0):GetRGBA()
      assert(r == 0 and g == 1 and b == 0 and a == 1)
      local object = CreateFromMixins({value=1}, {value=2, other=3})
      assert(object.value == 2 and object.other == 3)
      assert(tInvert({WARRIOR='Warrior'}).Warrior == 'WARRIOR')
    "#,
        )
        .unwrap();
}

#[test]
fn localized_class_names_come_from_the_host_and_are_isolated() {
    let script = extended();
    super::install_class_names(&script, [("WARRIOR", "Krieger"), ("MAGE", "Magier")]).unwrap();
    script
        .run("assert(tInvert(LOCALIZED_CLASS_NAMES_MALE).Krieger == 'WARRIOR')")
        .unwrap();
    let stock = UiScript::new().unwrap();
    stock
        .run("assert(LOCALIZED_CLASS_NAMES_MALE == nil)")
        .unwrap();
}

#[test]
fn modern_handlers_and_hooks_preserve_nil_payloads_and_legacy_globals() {
    let mut script = extended();
    script
        .run(
            r#"
      f = CreateFrame('Frame')
      f:RegisterEvent('EXAMPLE')
      calls = ''
      f:SetScript('OnEvent', function(self, kind, first, missing, last)
        assert(self == f and this == f and event == kind)
        assert(first == arg1 and missing == nil and last == arg3)
        assert(select('#', self, kind, first, missing, last) == 5)
        calls = calls .. 'a'
      end)
      f:HookScript('OnEvent', function(self, kind, ...)
        assert(self == f and kind == 'EXAMPLE' and select('#', ...) == 3)
        calls = calls .. 'b'
      end)
      f:HookScript('OnEvent', function() assert(this == f); calls = calls .. 'c' end)
      this, event, arg1, arg3 = 'outer', 'outer event', 'outer first', 'outer last'
    "#,
        )
        .unwrap();
    script.fire_event(
        "EXAMPLE",
        vec![
            ScriptValue::Str("first".into()),
            ScriptValue::Nil,
            ScriptValue::Str("last".into()),
        ],
    );
    script.run("assert(calls == 'abc' and this == 'outer' and event == 'outer event' and arg1 == 'outer first' and arg3 == 'outer last')").unwrap();
    assert!(script.errors().is_empty(), "{:?}", script.errors());
    let mut stock = UiScript::new().unwrap();
    // A stock VM registers only a name its event table has.
    stock.run("f=CreateFrame('Frame'); f:RegisterEvent('PLAYER_ENTERING_WORLD'); f:SetScript('OnEvent', function(...) stockArgs=arg.n end)").unwrap();
    stock.fire_event("PLAYER_ENTERING_WORLD", vec![]);
    assert_eq!(stock.eval::<i64>("return stockArgs").unwrap(), 0);
}

#[test]
fn widgets_reach_existing_and_future_frames_regions_and_font_strings() {
    let script = UiScript::new().unwrap();
    script
        .run("early = CreateFrame('Frame'); assert(early.SetSize == nil)")
        .unwrap();
    super::install(&script).unwrap();
    script
        .run(
            r#"
      early:SetSize(120, 45)
      -- Sizes are kept in layout units, as the reference keeps them: equal to a float's step.
      local function near(a, b) return math.abs(a - b) < .001 end
      local w,h = early:GetSize(); assert(near(w, 120) and near(h, 45))
      early:SetShown(false); assert(not early:IsShown())
      early:SetShown(true); assert(early:IsShown())
      local bar = CreateFrame('StatusBar', nil, early)
      bar:SetSize(50, 10); assert(near(bar:GetWidth(), 50))
      local texture = early:CreateTexture()
      texture:SetSize(20, 30); texture:SetShown(false); assert(not texture:IsShown())
      local text = early:CreateFontString()
      -- A font string with no font raises on SetText, as on 1.12.1 (#1470).
      text:SetFont('Fonts\\FRIZQT__.TTF', 12)
      text:SetSize(40, 15); text:SetFormattedText('%2$s %1$s', 'world', 'hello')
      assert(text:GetText() == 'hello world')
    "#,
        )
        .unwrap();
    UiScript::new()
        .unwrap()
        .run("assert(CreateFrame('Frame').SetSize == nil and hooksecurefunc == nil)")
        .unwrap();
}

#[test]
fn post_hooks_chain_without_losing_arguments_or_return_values() {
    let script = extended();
    script.run(r#"
      calls = ''
      function example(...)
        assert(select('#', ...) == 3 and select(2, ...) == nil)
        calls = calls .. 'a'; return 'one', nil, 'three', nil
      end
      hooksecurefunc('example', function(...) assert(select('#', ...) == 3); calls = calls .. 'b' end)
      hooksecurefunc('example', function() calls = calls .. 'c' end)
      local function check(...) assert(select('#', ...) == 4 and select(2, ...) == nil and select(3, ...) == 'three') end
      check(example('first', nil, 'last')); assert(calls == 'abc')
      local object = { value = 12, get = function(self) return self.value end }
      hooksecurefunc(object, 'get', function(self) assert(self == object) end)
      assert(object:get() == 12)
      assert(not pcall(hooksecurefunc, 'next', function() end))
      assert(not pcall(hooksecurefunc, 'missing', function() end))
      function broken() error('original failed') end
      local after = false
      hooksecurefunc('broken', function() after = true end)
      assert(not pcall(broken) and not after)
    "#).unwrap();
}

#[test]
fn unit_guid_uses_host_state_and_stock_target_hops() {
    let mut script = extended();
    let mut guids = benilla_ui::script::UnitGuids {
        player: 1,
        target: 0xF130_0000_0100_0012,
        ..Default::default()
    };
    guids.held.insert(guids.target, guids.player);
    script.set_unit_guids(&guids);
    script.run("assert(UnitGUID('PLAYER') == '0x0000000000000001' and UnitGUID('targettarget') == UnitGUID('player') and UnitGUID('pet') == nil)").unwrap();
}

#[test]
fn unit_event_filters_keep_registration_kind_and_clear_on_unregistration() {
    let mut script = extended();
    script
        .run(
            r#"
      f = CreateFrame('Frame'); seen = 0
      f:SetScript('OnEvent', function() seen = seen + 1 end)
      assert(f:RegisterUnitEvent('EXAMPLE', 'player', 'TARGET'))
      local registered, a, b = f:IsEventRegistered('EXAMPLE')
      assert(registered and a == 'player' and b == 'TARGET')
      f:RegisterEvent('EXAMPLE') -- keeps the unit filter
    "#,
        )
        .unwrap();
    script.fire_event("EXAMPLE", vec![ScriptValue::Str("party1".into())]);
    script.fire_event("EXAMPLE", vec![ScriptValue::Str("target".into())]);
    script.fire_event("EXAMPLE", vec![ScriptValue::Number(12.0)]);
    script.run("assert(seen == 2); f:UnregisterEvent('EXAMPLE'); assert(not f:IsEventRegistered('EXAMPLE')); f:RegisterEvent('EXAMPLE'); f:RegisterUnitEvent('EXAMPLE','player')").unwrap();
    script.fire_event("EXAMPLE", vec![ScriptValue::Str("party1".into())]);
    script
        .run("assert(seen == 3); f:UnregisterAllEvents(); f:RegisterUnitEvent('EXAMPLE','player')")
        .unwrap();
    script.fire_event("EXAMPLE", vec![ScriptValue::Str("party1".into())]);
    assert_eq!(script.eval::<i64>("return seen").unwrap(), 3);
    assert!(script.errors().is_empty(), "{:?}", script.errors());
}

#[test]
fn addon_scope_reaches_xml_inclusions_and_restores_nested_addon_context() {
    let script = extended();
    let outer = benilla_ui::script::pfui_host::addon_scope(script.lua(), "Outer").unwrap();
    script
        .run_chunk_named(
            b"local name, p = ...; p.name=name; p.count=1",
            "@Outer/first.lua",
        )
        .unwrap();
    {
        let _inner = benilla_ui::script::pfui_host::addon_scope(script.lua(), "Inner").unwrap();
        script
            .run_chunk_named(
                b"local name, p = ...; assert(name=='Inner' and p.count==nil); p.count=10",
                "@Inner/file.lua",
            )
            .unwrap();
    }
    let doc = benilla_ui::framexml::parse(
        r#"<Ui><Script file="one.lua"/><Include file="nested/child.xml"/></Ui>"#,
    )
    .unwrap();
    let report = benilla_ui::loader::load_in(&script, &doc, "Outer/root.xml", &|path| match path {
        "Outer/one.lua" => {
            Some(b"local name,p=...; assert(name==p.name and p.count==1); p.count=2".to_vec())
        }
        "Outer/nested/child.xml" => Some(b"<Ui><Script file=\"../two.lua\"/></Ui>".to_vec()),
        "Outer/two.lua" => {
            Some(b"local name,p=...; assert(name=='Outer' and p.count==2); p.count=3".to_vec())
        }
        _ => None,
    });
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    assert!(
        report.missing_files.is_empty(),
        "{:?}",
        report.missing_files
    );
    script
        .run_chunk_named(
            b"local name,p=...; assert(name=='Outer' and p.count==3)",
            "@Outer/last.lua",
        )
        .unwrap();
    drop(outer);
    script
        .run_chunk_named(b"assert(select('#',...) == 0)", "@ordinary.lua")
        .unwrap();
}

#[test]
fn server_calendar_clock_handles_leap_days_and_has_no_prelogin_value() {
    let script = extended();
    script
        .run("assert(C_DateAndTime.GetServerTimeLocal() == nil)")
        .unwrap();
    // 2024-02-29 23:59, with zero-based packet month/day fields.
    let epoch = crate::clock::epoch(24 * 372 + 31 + 28, 1439).unwrap();
    assert_eq!(epoch, 1_709_251_140.0);
    crate::clock::update(script.lua(), Some(epoch + 60.5)).unwrap();
    script.run("assert(date('!%Y-%m-%d %H:%M', C_DateAndTime.GetServerTimeLocal()) == '2024-03-01 00:00')").unwrap();
    assert!(crate::clock::epoch(23 * 372 + 31 + 28, 0).is_none());
    assert!(crate::clock::epoch(24 * 372, 1440).is_none());
    assert!(crate::clock::update(script.lua(), Some(f64::NAN)).is_err());
    crate::clock::update(script.lua(), None).unwrap();
    script
        .run("assert(C_DateAndTime.GetServerTimeLocal() == nil)")
        .unwrap();
}

#[test]
fn callbacks_keep_owner_nil_payload_and_defer_new_subscriptions() {
    let script = extended();
    script
        .run(
            r#"
      registry = CreateFromMixins(CallbackRegistryMixin)
      registry:OnLoad()
      registry:SetUndefinedEventsAllowed(true)
      local owner = {}
      local calls = 0
      registry:RegisterCallback('bag:closed', function(self, object, missing, final)
        assert(self == owner and object == 'bag' and missing == nil and final == 'last')
        calls = calls + 1
        registry:RegisterCallback('bag:closed', function() calls = calls + 100 end, 'late')
      end, owner)
      registry:TriggerEvent('bag:closed', 'bag', nil, 'last')
      assert(calls == 1)
      registry:UnregisterAllCallbacks(owner)
      registry:TriggerEvent('bag:closed', 'bag', nil, 'last')
      assert(calls == 101)
    "#,
        )
        .unwrap();
}

#[test]
fn continuations_are_once_and_login_callbacks_work_before_and_after_login() {
    let mut script = extended();
    script
        .run(
            r#"
      loginCalls, addonCalls = 0, 0
      EventUtil.ContinueOnPlayerLogin(function() loginCalls = loginCalls + 1 end)
      EventUtil.ContinueOnAddOnLoaded('Example', function() addonCalls = addonCalls + 1 end)
    "#,
        )
        .unwrap();
    script.fire_event("ADDON_LOADED", vec![ScriptValue::Str("Other".into())]);
    script.fire_event("ADDON_LOADED", vec![ScriptValue::Str("example".into())]);
    script.fire_event("ADDON_LOADED", vec![ScriptValue::Str("Example".into())]);
    script.fire_event("PLAYER_LOGIN", vec![]);
    script.fire_event("PLAYER_LOGIN", vec![]);
    script.run("EventUtil.ContinueOnPlayerLogin(function() loginCalls = loginCalls + 1 end); assert(loginCalls == 2 and addonCalls == 1)").unwrap();
    assert!(script.errors().is_empty(), "{:?}", script.errors());
}

/// `--eval`'s chunk runs last in the audit VM and its return value lands in the report: a value as
/// text, `nil` as "nil", a Lua error as "error: ...". Without data the VM holds the layer only.
#[cfg(feature = "audit")]
#[test]
fn an_audit_probe_reports_its_return_value() {
    let root = std::env::temp_dir().join(format!("benilla-audit-probe-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("pfUI_Turtle.toc"),
        "## Interface: 11200\npfUI.lua\n",
    )
    .unwrap();
    std::fs::write(root.join("pfUI.lua"), "pfUI = {}\n").unwrap();
    let probe = |chunk| {
        crate::audit::inspect_probed(&root, None, true, &[], chunk)
            .unwrap()
            .probe
    };
    assert_eq!(probe(None), None);
    assert_eq!(
        probe(Some("return type(C_Timer) .. ' ' .. 2 + 3")).as_deref(),
        Some("table 5")
    );
    assert_eq!(probe(Some("return nil")).as_deref(), Some("nil"));
    let failed = probe(Some("error('boom')")).unwrap();
    assert!(
        failed.starts_with("error: ") && failed.contains("boom"),
        "{failed}"
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[cfg(feature = "audit")]
#[test]
fn scanner_ignores_comments_and_strings_and_keeps_value_references() {
    let names = crate::audit::namespace_references(
        br#"
      -- C_Fake.Comment()
      --[=[ C_Fake.LongComment() ]=]
      local text = "C_Fake.String()"
      local escaped = 'it\'s C_Fake.Escaped()'
      local long = [==[ C_Fake.LongString() ]==]
      local fn = C_Spell.GetSpellName
      C_Timer -- harmless comment
        . NewTicker(1, fn)
      EventUtil.ContinueOnPlayerLogin(fn)
    "#,
    );
    assert_eq!(
        names.into_iter().collect::<Vec<_>>(),
        vec![
            "C_Spell.GetSpellName",
            "C_Timer.NewTicker",
            "EventUtil.ContinueOnPlayerLogin",
        ]
    );
}

#[test]
fn item_queries_wait_for_real_cache_arrivals_and_preserve_the_wide_tuple() {
    use benilla_ui::script::ItemTemplateView;
    let mut script = extended();
    script
        .run(
            r#"
        local a,b,c,d,e,f,g = C_Item.GetItemInfoInstant('item:6948')
        assert(a == 6948 and b == nil and g == nil)
        assert(not C_Item.IsItemDataCachedByID(6948))
    "#,
        )
        .unwrap();
    assert!(
        script
            .take_item_asks(benilla_ui::script::ItemCallback::Extension)
            .is_empty(),
        "instant lookups never query"
    );
    script
        .run(
            r#"
        loaded = 0
        local item = Item:CreateFromItemID(6948)
        item:ContinueOnItemLoad(function()
            loaded = loaded + 1
            assert(item:GetItemName() == 'Hearthstone')
            assert(select('#', C_Item.GetItemInfo(6948)) == 18)
            assert(select(4, C_Item.GetItemInfo(6948)) == 1)
        end)
        assert(loaded == 0 and C_Item.GetItemInfo(6948) == nil)
        -- pfUI's inspect skin and loot history ask these before they wait for the data.
        assert(not item:IsItemEmpty() and not item:IsItemDataCached())
        assert(Item:CreateFromItemID(nil):IsItemEmpty() and Item:CreateFromItemID(0):IsItemEmpty())
    "#,
        )
        .unwrap();
    // Each getter's miss asks inside its call, with no dedupe; the cache asks the wire once.
    let mut asks = script.take_item_asks(benilla_ui::script::ItemCallback::Extension);
    asks.dedup();
    assert_eq!(asks, vec![6948]);
    benilla_ui::script::pfui_host::items::set_metadata(script.lua(), 6948, 1, 0);
    script.set_item_template(
        6948,
        ItemTemplateView {
            name: "Hearthstone".into(),
            stackable: 1,
            ..Default::default()
        },
    );
    script.run("assert(loaded == 1); Item:CreateFromItemID(6948):ContinueOnItemLoad(function() loaded=loaded+1 end); assert(loaded==2)").unwrap();
    script
        .run("assert(Item:CreateFromItemID(6948):IsItemDataCached())")
        .unwrap();
    assert!(script.errors().is_empty(), "{:?}", script.errors());
    let stock = &mut UiScript::new().unwrap();
    stock.run("arrivals=0; local f=CreateFrame('Frame'); f:RegisterEvent('GET_ITEM_INFO_RECEIVED'); f:SetScript('OnEvent', function() arrivals=arrivals+1 end)").unwrap();
    stock.set_item_template(6948, ItemTemplateView::default());
    stock.run("assert(arrivals==0)").unwrap();
}

#[test]
fn item_counts_include_cold_bank_objects_charges_and_suffixed_names() {
    use benilla_ui::script::pfui_host::items::{set_instances, Instance};
    let script = extended();
    set_instances(
        script.lua(),
        [
            (
                (0, 1),
                Instance {
                    guid: 1,
                    item_id: 10,
                    count: 2,
                    uses: 100,
                    name: Some("Wand of the Owl".into()),
                },
            ),
            (
                (-1, 1),
                Instance {
                    guid: 2,
                    item_id: 10,
                    count: 3,
                    uses: 6,
                    name: Some("Wand of the Owl".into()),
                },
            ),
            (
                (benilla_ui::script::EQUIPMENT_BAG, 18),
                Instance {
                    guid: 3,
                    item_id: 10,
                    count: 1,
                    uses: 50,
                    name: Some("Wand of the Owl".into()),
                },
            ),
        ],
    );
    script
        .run(
            r#"
        assert(C_Item.GetItemCount(10)==3)
        assert(C_Item.GetItemCount('item:10',true)==6)
        assert(C_Item.GetItemCount('WAND OF THE OWL',true,true)==156)
        assert(C_Item.GetItemCount('Wand')==0)
        assert(C_Container.GetContainerItemCharges(0,1)==100)
        assert(C_Container.GetContainerItemCharges(0,2)==nil)
    "#,
        )
        .unwrap();
    set_instances(script.lua(), []);
    script
        .run("assert(C_Item.GetItemCount(10,true)==0)")
        .unwrap();
}

#[test]
fn bag_operations_use_the_paced_queue_and_openables_follow_instance_locks() {
    use benilla_ui::script::{ContainerMove, ContainerSlot, ContainerState, ItemTemplateView};
    let mut script = extended();
    script.set_item_template(
        10,
        ItemTemplateView {
            flags: 4,
            lock_id: 1,
            ..Default::default()
        },
    );
    let mut bag = ContainerState {
        num_slots: 3,
        ..Default::default()
    };
    bag.slots.insert(
        1,
        ContainerSlot {
            guid: 0x4000_0000_0000_0031,
            item_id: 10,
            count: 5,
            ..Default::default()
        },
    );
    script.set_container(0, Some(bag.clone()));
    script
        .run(
            r#"
        local openable, canOpen=C_Container.IsContainerItemOpenable(0,1)
        assert(openable and not canOpen)
        assert(C_Container.IsContainerItemOpenable(0,2)==nil)
        assert(C_Item.GetItemID(ItemLocation:CreateFromBagAndSlot(0,1))==10)
        assert(not C_Container.SwapItems(0,1,0,1))
        assert(not C_Container.MoveItem(0,1,0,2,0))
        assert(not C_Container.MoveItem(0,1,0,2,256))
        assert(not C_Container.SwapItems(0,2,0,1))
        assert(not C_Container.SwapItems(0,1,0,4))
        assert(C_Container.MoveItem(0,1,0,2,2))
    "#,
        )
        .unwrap();
    assert_eq!(
        benilla_ui::script::pfui_host::items::take_bag_moves(script.lua()),
        [ContainerMove {
            src_bag: 0,
            src_slot: 1,
            dst_bag: 0,
            dst_slot: 2,
            count: Some(2)
        }]
    );
    bag.slots.get_mut(&1).unwrap().flags = 4;
    let locked = bag.slots.get(&1).unwrap().guid;
    script.set_container(0, Some(bag));
    script.lock_item(locked);
    script.run("local _,canOpen=C_Container.IsContainerItemOpenable(0,1); assert(canOpen); assert(not C_Container.SwapItems(0,1,0,2))").unwrap();
    assert!(benilla_ui::script::pfui_host::items::take_bag_moves(script.lua()).is_empty());
}

#[test]
fn config_copies_and_icon_pools_preserve_ownership_across_reuse() {
    let script = extended();
    script
        .run(
            r#"
        assert(Round(0.5)==1 and Round(-0.5)==-1 and Round(-1.49)==-1)
        assert(Clamp(4,0,1)==1 and Clamp(-4,0,1)==0)
        local original={profile={font='12'}}; original.self=original
        local copy=CopyTable(original)
        assert(copy~=original and copy.profile~=original.profile and copy.self==copy)
        copy.profile.font='14'; assert(original.profile.font=='12')
        assert(CopyTable(original,true).profile==original.profile)
        local created,resets=0,0
        local pool=CreateObjectPool(function() created=created+1; return {} end,
          function(_,o) resets=resets+1; o.value=nil end)
        local first,new=pool:Acquire(); assert(new and resets==1)
        first.value=12
        local second=pool:Acquire()
        assert(pool:GetNumActive()==2)
        local count=0; for o in pool:EnumerateActive() do count=count+1 end
        assert(count==2 and not pool:Release({}))
        assert(pool:Release(first) and not pool:Release(first))
        local reused,fresh=pool:Acquire()
        assert(reused==first and not fresh and reused.value==nil and created==2)
        pool:ReleaseAll(); assert(pool:GetNumActive()==0)
        local texturePool=CreateTexturePool(CreateFrame('Frame'),'BORDER')
        local texture=texturePool:Acquire(); texture:Show()
        texturePool:ReleaseAll(); assert(not texture:IsShown())
    "#,
        )
        .unwrap();
}
