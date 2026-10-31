//! Gated connections from the optional pfUI extension to the stock VM's state.
//! A feature-enabled build still uses the stock ABI until the extension installs its VM flag.

use mlua::{Function, IntoLuaMulti, Lua, MultiValue, Table, Value};

mod addons;
pub mod attributes;
pub mod auras;
mod bubbles;
pub mod casts;
mod cursor;
mod descriptors;
pub(crate) mod equipment_actions;
pub mod inline_texture;
pub mod items;
pub mod maps;
mod merchant;
pub mod player_stats;
pub mod quests;
pub mod social;
pub(crate) mod tooltip_subjects;
pub mod totems;
pub mod units;

/// The key a frame registers under for an event the 1.12 table lacks: the name as given, once the
/// extension is installed in an in-game VM. ClassicAPI's events (`PLAYER_SWING`, the modern aura
/// and cast events) are the extension's to fire, and the stock table would drop their listeners.
pub(crate) fn extension_event(model: &super::Model, name: &str) -> Option<String> {
    (model.pfui_cursor.is_some() && model.vm == super::glue::VmKind::InGame)
        .then(|| name.to_string())
}

/// Timer callbacks are protected individually, but errors still belong in the VM error stream.
pub fn report_error(lua: &Lua, message: String) {
    lua.app_data_mut::<super::Model>()
        .expect("model")
        .record_error(message.clone());
}

/// Extension-owned event payloads use the stock protected dispatcher and legacy argN globals.
/// Every pfUI raise comes this way. ClassicAPI and Nampower raise these events from their own
/// per-frame hooks, part of the client's frame, so a raise here is the tick's
/// ([`super::origin::Origin::Tick`]).
pub fn fire_pfui_event(lua: &Lua, event: &str, args: Vec<super::ScriptValue>) {
    super::origin::within(super::origin::Origin::Tick, || {
        super::tick::fire_event_into(lua, event, args);
    });
}

/// The spell `text` names, `Name(Rank N)`, in the player's book and then the pet's, as
/// `CastSpellByName` resolves it, against the books as they stand at the call.
pub(crate) fn spell_named(lua: &Lua, text: &str) -> Option<super::NamedSpell> {
    let book = super::live::ask(lua, &super::BookNow);
    let pet = super::live::ask(lua, &super::PetBookNow);
    let model = lua.app_data_ref::<super::Model>().expect("model");
    super::resolve_spell_in_books(
        book.as_ref().unwrap_or(&model.spellbook),
        pet.as_ref().unwrap_or(&model.pet_book),
        text,
    )
}

/// The item `token`'s unit shows at live-API id `slot`, as the `GetInventoryItem*` getters read
/// it: the token resolved at the call, then the player's own table or another player's visible
/// items.
pub(crate) fn inv_slot(
    lua: &Lua,
    model: &super::Model,
    token: &str,
    slot: usize,
) -> Option<super::InvSlotView> {
    let reader = super::char_stats::inventory_reader(lua, model, token)?;
    model.inv_slot_as(lua, reader, slot)
}

/// Queue `calls` for the host, in order, under the input events running now: made while the host
/// applies a call, they apply right after it, as a click action's steps (the target selected, the
/// cast, the selection put back) each apply as their own call.
pub fn queue_calls(script: &mut super::UiScript, calls: Vec<super::ScriptCall>) {
    let mut model = script.model_mut();
    for call in calls {
        model.queue_call(call);
    }
}

/// Whether `benilla-pfui::install` ran on this VM.
pub(crate) fn installed(lua: &Lua) -> mlua::Result<bool> {
    Ok(lua.named_registry_value::<Option<bool>>("benilla.pfui.installed")? == Some(true))
}

/// A handler call on pfUI's VM, by the stock convention (`this` and `arg1..` set around the call,
/// under the error function) with the modern arguments as well: `self`, then `event` for an
/// `OnEvent` (the walk's global), then `arg1..`. A unit event a frame filtered to other units
/// ([`RegisterUnitEvent`]) is dropped.
pub(crate) fn invoke_handler(
    lua: &Lua,
    wrapper: Table,
    handler: &Function,
    is_event: bool,
    extra: Vec<Value>,
) -> mlua::Result<()> {
    let g = lua.globals();
    let event = match is_event {
        true => match g.get::<Value>("event")? {
            Value::String(s) => Some(s),
            _ => None,
        },
        false => None,
    };
    if let (Some(event), Some(Value::String(unit))) = (&event, extra.first()) {
        let filters: Table = lua.named_registry_value("benilla.pfui.unit_events")?;
        if let Value::Table(events) = filters.raw_get::<Value>(wrapper.clone())? {
            if let Value::Table(units) = events.raw_get::<Value>(event.clone())? {
                let unit = unit.to_string_lossy();
                let matches = units
                    .sequence_values::<String>()
                    .collect::<mlua::Result<Vec<_>>>()?
                    .iter()
                    .any(|allowed| allowed.eq_ignore_ascii_case(&unit));
                if !matches {
                    return Ok(());
                }
            }
        }
    }
    let mut names = vec!["this".to_string()];
    names.extend((1..=extra.len()).map(|i| format!("arg{i}")));
    let saved: Vec<Value> = names
        .iter()
        .map(|name| g.get::<Value>(name.as_str()))
        .collect::<mlua::Result<_>>()?;
    let mut args = vec![Value::Table(wrapper.clone())];
    args.extend(event.map(Value::String));
    args.extend(extra.iter().cloned());
    g.set("this", wrapper)?;
    for (name, v) in names.iter().skip(1).zip(extra) {
        g.set(name.as_str(), v)?;
    }
    let error_function = super::stdlib::error_function(lua);
    let outcome = super::origin::within(super::origin::Origin::Script, || {
        super::stdlib::with_capture(|| {
            let args = MultiValue::from_vec(args);
            match &error_function {
                Some(h) => handler.call_under::<()>(h, args),
                None => handler.call::<()>(args),
            }
        })
    });
    for (name, v) in names.iter().zip(saved) {
        g.set(name.as_str(), v)?;
    }
    outcome
}

/// Publish the GUID resolved by the stock unit resolver. Its case folding and target hops are
/// reused, while the extension separately owns additional unit-token kinds.
pub fn install(lua: &Lua) -> mlua::Result<()> {
    let globals = lua.globals();
    if !globals.get::<Value>("UnitGUID")?.is_nil() {
        return Err(mlua::Error::runtime("UnitGUID already installed"));
    }
    globals.set(
        "UnitGUID",
        lua.create_function(|lua, token: String| {
            let guid = match units::custom_guid(lua, &token) {
                Some(guid) => guid,
                None => {
                    let model = lua.app_data_ref::<super::Model>().expect("model");
                    super::unit::guid_of(lua, &model, &token)?
                }
            };
            Ok(guid
                .filter(|guid| *guid != 0)
                .map(|guid| format!("0x{guid:016X}")))
        })?,
    )?;
    let methods: Table = lua.named_registry_value(super::REG_FRAME_METHODS)?;
    methods.set(
        "IsEventRegistered",
        lua.create_function(|lua, (frame, event): (Table, String)| {
            let handle = super::object::frame_handle_of(lua, &frame)?;
            let registered = lua
                .app_data_ref::<super::Model>()
                .expect("model")
                .frame_events
                .get(&handle)
                .is_some_and(|events| events.contains(&event));
            let mut result = vec![Value::Boolean(registered)];
            if registered {
                let filters: Table = lua.named_registry_value("benilla.pfui.unit_events")?;
                if let Value::Table(events) = filters.raw_get::<Value>(frame)? {
                    if let Value::Table(units) = events.raw_get::<Value>(event)? {
                        result.extend(
                            units
                                .sequence_values::<Value>()
                                .collect::<mlua::Result<Vec<_>>>()?,
                        );
                    }
                }
            }
            Ok(MultiValue::from_vec(result))
        })?,
    )?;
    globals.set("INTERFACE_VERSION", super::addon_gate::CLIENT_INTERFACE)?;
    items::install(lua)?;
    cursor::install(lua)?;
    merchant::install(lua)?;
    addons::install(lua)?;
    bubbles::install(lua)?;
    descriptors::install(lua)?;
    social::install(lua)?;
    units::install(lua)?;
    player_stats::install(lua)?;
    totems::install(lua)?;
    super::nameplate::pfui::install(lua)?;
    attributes::install(lua)?;
    auras::install(lua)?;
    super::tooltip_spell::pfui::install(lua)?;
    tooltip_subjects::install(lua)?;
    super::tooltip_item::pfui::install(lua)?;
    quests::install(lua)?;
    super::macros::pfui::install(lua)?;
    super::reputation::pfui::install(lua)?;
    Ok(())
}

pub(super) fn item_received(lua: &Lua, item_id: u32) {
    item_result(lua, item_id, true);
}

/// The host calls this for a server's negative item-query answer as well as successful loads.
pub fn item_result(lua: &Lua, item_id: u32, success: bool) {
    if lua
        .named_registry_value::<Option<bool>>("benilla.pfui.installed")
        .ok()
        .flatten()
        == Some(true)
    {
        fire_pfui_event(
            lua,
            "GET_ITEM_INFO_RECEIVED",
            vec![
                super::ScriptValue::Number(f64::from(item_id)),
                super::ScriptValue::Bool(success),
            ],
        );
    }
}

/// A load scope shared by TOC files, XML inclusions and synchronous LoadAddOn calls. Restoring
/// the enclosing scope keeps a dependent addon's private table from leaking into its caller.
pub struct AddonScope<'a> {
    lua: &'a Lua,
    previous: Value,
    active: bool,
}

impl Drop for AddonScope<'_> {
    fn drop(&mut self) {
        if self.active {
            let _ = self
                .lua
                .set_named_registry_value("benilla.pfui.addon_context", self.previous.clone());
        }
    }
}

pub fn addon_scope<'a>(lua: &'a Lua, name: &str) -> mlua::Result<AddonScope<'a>> {
    let active = lua.named_registry_value::<Option<bool>>("benilla.pfui.installed")? == Some(true);
    let previous: Value = lua.named_registry_value("benilla.pfui.addon_context")?;
    if active {
        let privates: Table = lua.named_registry_value("benilla.pfui.addon_privates")?;
        let private = match privates.raw_get::<Value>(name)? {
            Value::Table(table) => table,
            _ => {
                let table = lua.create_table()?;
                privates.raw_set(name, table.clone())?;
                table
            }
        };
        let context = lua.create_table()?;
        context.set(1, name)?;
        context.set(2, private)?;
        lua.set_named_registry_value("benilla.pfui.addon_context", context)?;
        // Stock globals a preceding load assigned are adapted before this addon reads them.
        if let Some(bridge) =
            lua.named_registry_value::<Option<Table>>("benilla.pfui.globals_bridge")?
        {
            if let Some(adapt) = bridge.get::<Option<Function>>("beforeAddon")? {
                adapt.call::<()>(())?;
            }
        }
    }
    Ok(AddonScope {
        lua,
        previous,
        active,
    })
}

/// The varargs an addon file's chunk runs with: `(addonName, privateTable)` inside an addon's
/// scope ([`addon_scope`]), none elsewhere.
pub(crate) fn chunk_args(lua: &Lua) -> Result<MultiValue, (mlua::Error, Vec<u8>)> {
    let args = || -> mlua::Result<MultiValue> {
        match lua.named_registry_value("benilla.pfui.addon_context")? {
            Value::Table(context) => {
                (context.get::<String>(1)?, context.get::<Table>(2)?).into_lua_multi(lua)
            }
            _ => Ok(MultiValue::new()),
        }
    };
    args().map_err(|e| (e, Vec::new()))
}
