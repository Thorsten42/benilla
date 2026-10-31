//! The engine half the ClassicAPI DLL's own Lua addon (`!!!ClassicAPI`) builds on, and the
//! FrameXML it reads: each behaviour as that addon or pfUI uses it.

use benilla_ui::script::UiScript;

fn extended() -> UiScript {
    let mut script = UiScript::new().unwrap();
    script.keep_calls();
    super::install(&script).unwrap();
    script
}

fn load(script: &UiScript, xml: &str) {
    let doc = benilla_ui::framexml::parse(xml).expect("parses");
    let report = benilla_ui::loader::load(script, &doc, &|_| None);
    assert!(report.errors.is_empty(), "load: {:?}", report.errors);
}

#[test]
fn strsplit_splits_on_any_separator_character_and_caps_the_pieces() {
    let script = extended();
    script
        .run(
            r#"
        local a, b, c = strsplit(",", "a,b,c")
        assert(a == "a" and b == "b" and c == "c")
        local x, y = strsplit(",", "a,b,c,d", 2)
        assert(x == "a" and y == "b,c,d")
        local p, q, r = strsplit(" -", "a b-c")
        assert(p == "a" and q == "b" and r == "c")
        local e1, e2, e3 = strsplit(",", "a,,b")
        assert(e1 == "a" and e2 == "" and e3 == "b")
        assert(strsplit(",", "whole", 1) == "whole")
        local t = strsplittable(":", "hello:world:banana")
        assert(table.getn(t) == 3 and t[3] == "banana")
        assert(string.split == strsplit)
    "#,
        )
        .unwrap();
}

#[test]
fn securecall_reports_an_error_to_the_handler_and_goes_on() {
    let script = extended();
    script
        .run(
            r#"
        local reported = {}
        seterrorhandler(function(message) table.insert(reported, message) end)
        assert(securecall(math.max, 1, 9) == 9)
        assert(securecall(function() error("boom") end) == nil)
        assert(table.getn(reported) == 1)
        local seen = {}
        secureexecuterange({10, 20}, function(key, value, extra)
          if value == 10 then error("first fails") end
          seen[key] = value .. extra
        end, "x")
        assert(seen[2] == "20x" and table.getn(reported) == 2)
        assert(not pcall(securecallfunction, "notAFunction"))
    "#,
        )
        .unwrap();
}

#[test]
fn ui_colors_are_built_through_the_callers_create_color() {
    let script = extended();
    script
        .run(
            r#"
        local made = 0
        local create = CreateColor
        CreateColor = function(r, g, b, a) made = made + 1; return create(r, g, b, a) end
        local rows = C_UIColor.GetColors()
        assert(table.getn(rows) > 300 and made == table.getn(rows))
        local normal
        for _, row in ipairs(rows) do
          if row.baseTag == "NORMAL_FONT_COLOR" then normal = row.color end
        end
        assert(normal.r == 1 and math.abs(normal.g - 0.8235) < 0.001 and normal.b == 0)
        CreateColor = create
    "#,
        )
        .unwrap();
}

#[test]
fn the_color_code_reads_the_color_itself_after_the_addon_replaces_the_mixin() {
    let script = extended();
    // !!!ClassicAPI's ColorMixin asks the engine for its code: the two must not call each other.
    script
        .run(
            r#"
        ColorMixin = {}
        function ColorMixin:GenerateHexColor() return C_ColorUtil.GenerateTextColorCode(self) end
        local color = {r = 1, g = 0.2, b = 0, a = 1}
        assert(ColorMixin.GenerateHexColor(color) == "ffff3300")
    "#,
        )
        .unwrap();
}

#[test]
fn the_classicapi_table_keeps_the_engine_functions_an_addon_replaces() {
    let script = extended();
    script
        .run(
            r#"
        assert(ClassicAPI.strsplit == strsplit and ClassicAPI.C_CVar.GetCVarBool == C_CVar.GetCVarBool)
        local engine = IsModifiedClick
        IsModifiedClick = function() return "addon" end
        assert(ClassicAPI.IsModifiedClick == engine)
    "#,
        )
        .unwrap();
}

#[test]
fn an_unknown_cvar_reads_nil_and_a_known_one_reads_as_a_boolean() {
    let script = extended();
    script.register_cvars([("benillaFlag", "1"), ("benillaOff", "0")]);
    script
        .run(
            r#"
        assert(GetCVar("noSuchCVar") == nil)
        assert(C_CVar.GetCVarBool("noSuchCVar") == nil)
        assert(C_CVar.GetCVarBool("benillaFlag") == true and C_CVar.GetCVarBool("benillaOff") == false)
        assert(C_CVar.DoesCVarExist("benillaFlag") and not C_CVar.DoesCVarExist("noSuchCVar"))
    "#,
        )
        .unwrap();
}

#[test]
fn modified_clicks_have_the_documented_actions_and_defaults() {
    let script = extended();
    script
        .run(
            r#"
        assert(GetModifiedClick("SELFCAST") == "ALT" and GetModifiedClick("CHATLINK") == "SHIFT-BUTTON1")
        assert(GetNumModifiedClickActions() == 18)
        SetModifiedClick("SELFCAST", "ctrl")
        assert(GetModifiedClick("SELFCAST") == "CTRL")
        assert(not pcall(SetModifiedClick, "NOSUCHACTION", "ALT"))
        assert(IsModifiedClick("SELFCAST") == nil)
    "#,
        )
        .unwrap();
}

#[test]
fn class_names_fill_a_table_the_addon_rebuilds_the_global_with() {
    let script = extended();
    super::install_class_names(&script, [("WARRIOR", "Warrior"), ("MAGE", "Mage")]).unwrap();
    script
        .run(
            r#"
        LOCALIZED_CLASS_NAMES_MALE = {}
        FillLocalizedClassList(LOCALIZED_CLASS_NAMES_MALE, false)
        assert(LOCALIZED_CLASS_NAMES_MALE.MAGE == "Mage")
        local keep = {OTHER = 1}
        assert(FillLocalizedClassList(keep, true) == keep and keep.WARRIOR == "Warrior" and keep.OTHER == 1)
    "#,
        )
        .unwrap();
}

#[test]
fn parent_keys_mixins_and_key_values_reach_the_object_before_its_onload() {
    let script = extended();
    script
        .run("UIParent = CreateFrame('Frame', 'UIParent')")
        .unwrap();
    script
        .run(
            r#"
        TestMixin = {}
        function TestMixin:OnLoad()
          loaded = {self.Child ~= nil, self.Child.Art ~= nil, self.label, self.size, self.flag}
        end
    "#,
        )
        .unwrap();
    load(
        &script,
        r#"<Ui>
          <Frame name="TestBase" mixin="TestMixin" virtual="true">
            <KeyValues>
              <KeyValue key="label" value="text"/>
              <KeyValue key="size" value="12" type="number"/>
              <KeyValue key="flag" value="true" type="boolean"/>
            </KeyValues>
            <Frames>
              <Frame parentKey="Child">
                <Layers><Layer><Texture parentKey="Art"/></Layer></Layers>
              </Frame>
            </Frames>
            <Scripts><OnLoad method="OnLoad"/></Scripts>
          </Frame>
          <Frame name="TestFrame" inherits="TestBase" parent="UIParent"/>
        </Ui>"#,
    );
    script
        .run(
            r#"
        assert(loaded[1] and loaded[2] and loaded[3] == "text" and loaded[4] == 12 and loaded[5] == true)
        assert(TestFrame.Child:GetParent() == TestFrame)
    "#,
        )
        .unwrap();
}

#[test]
fn an_intrinsic_tag_runs_its_handlers_around_the_frames_own() {
    let script = extended();
    script
        .run("UIParent = CreateFrame('Frame', 'UIParent'); order = {}")
        .unwrap();
    script
        .run(
            r#"
        TestIntrinsicMixin = {}
        function TestIntrinsicMixin:OnLoad_Intrinsic() table.insert(order, "intrinsic load") end
        function TestIntrinsicMixin:OnShow_Intrinsic() table.insert(order, "intrinsic show") end
    "#,
        )
        .unwrap();
    load(
        &script,
        r#"<Ui>
          <Frame name="TestEventFrame" mixin="TestIntrinsicMixin" intrinsic="true">
            <Scripts>
              <OnLoad method="OnLoad_Intrinsic"/>
              <OnShow method="OnShow_Intrinsic" intrinsicOrder="postcall"/>
            </Scripts>
          </Frame>
          <TestEventFrame name="TestInstance" parent="UIParent" hidden="true">
            <Scripts>
              <OnLoad>table.insert(order, "own load")</OnLoad>
              <OnShow>table.insert(order, "own show")</OnShow>
            </Scripts>
          </TestEventFrame>
        </Ui>"#,
    );
    script
        .run(
            r#"
        assert(TestInstance:GetObjectType() == "Frame" and _G.TestEventFrame == nil)
        TestInstance:Show()
        assert(table.concat(order, ",") == "intrinsic load,own load,own show,intrinsic show",
          table.concat(order, ","))
        local made = CreateFrame("TestEventFrame", nil, UIParent)
        assert(made.OnLoad_Intrinsic ~= nil)
    "#,
        )
        .unwrap();
}

#[test]
fn template_lists_apply_each_template_and_inherit_prepends_the_handler() {
    let script = extended();
    script
        .run("UIParent = CreateFrame('Frame', 'UIParent'); calls = {}")
        .unwrap();
    load(
        &script,
        r#"<Ui>
          <Frame name="TestA" virtual="true">
            <KeyValues><KeyValue key="a" value="1" type="number"/></KeyValues>
            <Scripts><OnShow>table.insert(calls, "A")</OnShow></Scripts>
          </Frame>
          <Frame name="TestB" virtual="true">
            <KeyValues><KeyValue key="b" value="2" type="number"/></KeyValues>
            <Scripts><OnShow inherit="prepend">table.insert(calls, "B")</OnShow></Scripts>
          </Frame>
        </Ui>"#,
    );
    script
        .run(
            r#"
        local f = CreateFrame("Frame", nil, UIParent, "TestA, TestB")
        assert(f.a == 1 and f.b == 2)
        calls = {}
        f:Hide(); f:Show()
        assert(table.concat(calls, ",") == "A,B", table.concat(calls, ","))
    "#,
        )
        .unwrap();
}

#[test]
fn a_relative_key_anchors_to_a_sibling_and_an_anchor_gives_its_own_offset() {
    let script = extended();
    script
        .run("UIParent = CreateFrame('Frame', 'UIParent')")
        .unwrap();
    load(
        &script,
        r#"<Ui>
          <Frame name="TestOuter" parent="UIParent">
            <Size x="100" y="100"/>
            <Anchors><Anchor point="CENTER"/></Anchors>
            <Frames>
              <Frame parentKey="First"><Size x="10"/></Frame>
              <Frame parentKey="Second">
                <Size x="10" y="10"/>
                <Anchors><Anchor point="TOP" relativeKey="$parent.First" relativePoint="BOTTOM" y="-5"/></Anchors>
              </Frame>
            </Frames>
          </Frame>
        </Ui>"#,
    );
    script
        .run(
            r#"
        local point, relative, relativePoint, x, y = TestOuter.Second:GetPoint(1)
        assert(relative == TestOuter.First and point == "TOP" and relativePoint == "BOTTOM")
        -- Offsets are kept in layout units.
        assert(x == 0 and math.abs(y + 5) < 0.001, tostring(x) .. "," .. tostring(y))
        assert(math.abs(TestOuter.First:GetWidth() - 10) < 0.001 and TestOuter.First:GetHeight() == 0,
          TestOuter.First:GetWidth() .. "x" .. TestOuter.First:GetHeight())
    "#,
        )
        .unwrap();
}

#[test]
fn an_atlas_points_a_texture_at_its_rectangle() {
    let script = extended();
    script
        .run(
            r#"
        C_Texture.RegisterAtlas("test-gem", "Interface\\Test\\sheet", 32, 16, 0, 0.25, 0.5, 1)
        local info = C_Texture.GetAtlasInfo("TEST-GEM")
        assert(info.filename == "Interface\\Test\\sheet" and info.width == 32 and info.rightTexCoord == 0.25)
        assert(C_Texture.GetAtlasExists("test-gem") and not C_Texture.GetAtlasExists("nope"))
        local t = CreateFrame("Frame"):CreateTexture()
        t:SetAtlas("test-gem", true)
        assert(t:GetAtlas() == "test-gem" and t:GetWidth() == 32 and t:GetHeight() == 16)
        t:SetAtlas("unknown")
        assert(t:GetAtlas() == "test-gem")
        t:SetTexture("Interface\\Other")
        assert(t:GetAtlas() == nil)
    "#,
        )
        .unwrap();
}

#[test]
fn a_cooldown_frame_is_the_indicator_model_with_the_modern_methods() {
    let script = extended();
    script
        .run("UIParent = CreateFrame('Frame', 'UIParent')")
        .unwrap();
    load(
        &script,
        r#"<Ui><Model name="CooldownFrameTemplate" setAllPoints="true" hidden="true" virtual="true"/></Ui>"#,
    );
    script
        .run(
            r#"
        local cd = CreateFrame("Cooldown", nil, UIParent, "CooldownFrameTemplate")
        assert(cd:IsObjectType("Model"))
        cd:SetReverse(true); cd:SetDrawBling(false)
        assert(cd:GetReverse() and not cd:GetDrawBling() and cd:GetDrawSwipe())
        cd:SetCooldown(100, 10)
        assert(cd:IsShown() and select(2, cd:GetCooldownTimes()) == 10000)
        cd:Clear()
        assert(not cd:IsShown() and cd:GetCooldownTimes() == 0)
    "#,
        )
        .unwrap();
}

#[test]
fn an_alpha_animation_fades_its_parent_and_reports_the_finish() {
    let mut script = extended();
    script
        .run(
            r#"
        frame = CreateFrame("Frame")
        frame:SetAlpha(1)
        group = frame:CreateAnimationGroup()
        local fade = group:CreateAnimation("Alpha")
        fade:SetChange(-1)
        fade:SetDuration(1)
        finished = 0
        group:SetScript("OnFinished", function() finished = finished + 1 end)
        group:Play()
        assert(group:IsPlaying())
    "#,
        )
        .unwrap();
    script.tick(0.5);
    script
        .run("assert(math.abs(frame:GetAlpha() - 0.5) < 0.01, frame:GetAlpha())")
        .unwrap();
    script.tick(0.6);
    // The target's own alpha comes back at the end, as it does without SetToFinalAlpha.
    script
        .run("assert(finished == 1 and not group:IsPlaying() and frame:GetAlpha() == 1)")
        .unwrap();
}
