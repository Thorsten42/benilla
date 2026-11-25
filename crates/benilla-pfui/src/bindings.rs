//! ClassicAPI's key-binding commands (`bindings/Bindings.xml`, spliced by `bindings/Inject.cpp`):
//! Set Focus, Target Focus, Interact With Mouseover, Interact With Target and AoE Loot, at the tail
//! of the stock file's TARGETING section, so the Key Bindings window lists them there and they
//! take their indices from the same counter as the stock commands. The stock file itself is
//! untouched; the commands join its parsed list before it registers ([`splice`]). Their labels are
//! `BINDING_NAME_*` strings, which ClassicAPI's own addon defines.

use benilla_ui::bindings_xml::{parse, Binding};

/// The commands, each body a Lua chunk as a stock binding's.
const COMMANDS: &str = r#"<Bindings>
<Binding name="FOCUSTARGET">FocusUnit("target")</Binding>
<Binding name="TARGETFOCUS">TargetUnit("focus")</Binding>
<Binding name="INTERACTMOUSEOVER">if not C_PlayerInteractionManager.InteractUnit("mouseover", false, false) then C_PlayerInteractionManager.InteractUnit("anyinteract") end</Binding>
<Binding name="INTERACTTARGET">C_PlayerInteractionManager.InteractUnit("anyinteract")</Binding>
<Binding name="AOELOOT">C_Loot.LootAllCorpses()</Binding>
</Bindings>"#;

/// The section the commands extend.
const SECTION: &str = "TARGETING";

/// `core` with the commands at the tail of its TARGETING section: before the next section's
/// first binding, or at the end when TARGETING is the last; a file with no TARGETING section, or
/// one that has a command already, is left as it is.
pub fn splice(mut core: Vec<Binding>) -> Vec<Binding> {
    let commands = parse(COMMANDS).expect("the layer's own commands parse");
    if commands
        .iter()
        .any(|c| core.iter().any(|b| b.name.eq_ignore_ascii_case(&c.name)))
    {
        return core;
    }
    let Some(start) = core
        .iter()
        .position(|b| b.header.as_deref() == Some(SECTION))
    else {
        return core;
    };
    let at = core[start + 1..]
        .iter()
        .position(|b| b.header.is_some())
        .map_or(core.len(), |offset| start + 1 + offset);
    core.splice(at..at, commands);
    core
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(bindings: &[Binding]) -> Vec<&str> {
        bindings.iter().map(|b| b.name.as_str()).collect()
    }

    /// The commands land after TARGETING's last binding and before the next section; with
    /// TARGETING last, at the end; with no TARGETING, nowhere.
    #[test]
    fn the_commands_join_the_targeting_section() {
        let core = parse(
            r#"<Bindings>
            <Binding name="MOVEFORWARD" header="MOVEMENT">x()</Binding>
            <Binding name="TARGETNEARESTENEMY" header="TARGETING">x()</Binding>
            <Binding name="TARGETSELF">x()</Binding>
            <Binding name="ACTIONBUTTON1" header="ACTIONBAR">x()</Binding>
            </Bindings>"#,
        )
        .unwrap();
        assert_eq!(
            names(&splice(core)),
            [
                "MOVEFORWARD",
                "TARGETNEARESTENEMY",
                "TARGETSELF",
                "FOCUSTARGET",
                "TARGETFOCUS",
                "INTERACTMOUSEOVER",
                "INTERACTTARGET",
                "AOELOOT",
                "ACTIONBUTTON1",
            ]
        );
        let last = parse(
            r#"<Bindings><Binding name="TARGETSELF" header="TARGETING">x()</Binding></Bindings>"#,
        )
        .unwrap();
        assert_eq!(names(&splice(last)).last(), Some(&"AOELOOT"));
        let none = parse(r#"<Bindings><Binding name="JUMP">x()</Binding></Bindings>"#).unwrap();
        assert_eq!(names(&splice(none)), ["JUMP"]);
        let spliced = splice(
            parse(r#"<Bindings><Binding name="TARGETSELF" header="TARGETING">x()</Binding></Bindings>"#)
                .unwrap(),
        );
        assert_eq!(
            splice(spliced.clone()),
            spliced,
            "a second splice adds nothing"
        );
    }
}
