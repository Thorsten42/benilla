//! The FrameXML a ClassicAPI client reads, as its own Lua addon (`!!!ClassicAPI`) and pfUI write
//! it.

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
