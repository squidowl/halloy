use itertools::Itertools;
use once_cell::sync::Lazy;
use rand::RngCore;
use rustpython_vm::PyObjectRef;
use rustpython_vm::{
    self as rpvm, Interpreter, builtins::PyStrRef, convert::ToPyObject,
    pymodule, scope::Scope,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Mutex;

use crate::buffer::{self, Upstream};
use crate::command::Command;
use crate::isupport::CaseMap;
use crate::target::Query;
use crate::{Config, User, history, input};

thread_local! {
    static ACTIONS: Lazy<Mutex<Vec<Option<HalloyAction>>>> =
        Lazy::new(|| Mutex::new(Vec::new()));
}
thread_local! {
    static PY_HOOKS: Lazy<RefCell<HashMap<u32, HalloyHook>>> = Lazy::new(|| RefCell::new(HashMap::new()));
}
thread_local! {
    // the printing queue
    static PY_PRINT: Lazy<RefCell<Vec<String>>> = Lazy::new(|| RefCell::new(Vec::new()));
}
thread_local! {
    static PY_PLUGINPREFS: Lazy<RefCell<HashMap<String, String>>> = Lazy::new(|| RefCell::new(HashMap::new()))
}
thread_local! {
    static PY_COMMAND_QUEUE: Lazy<RefCell<Vec<Command>>> = Lazy::new(|| RefCell::new(Vec::new()))
}
thread_local! {
    static PY_COMMANDS: Lazy<RefCell<HashMap<String, PyObjectRef>>> = Lazy::new(|| RefCell::new(HashMap::new()))
}

#[derive(Clone, Debug)]
pub enum HalloyHook {
    ClientCommand(RustpythonClientCommand),
    Print(RustpythonHookPrint),
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct RustpythonClientCommand {
    pub command_name: String,
    pub py_funct: PyObjectRef,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct RustpythonHookPrint {
    pub run_if: String,
    pub py_funct: PyObjectRef,
    pub attrs: Option<PyObjectRef>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct RustpythonHookPrintEmit {
    pub run_if: String,
    pub string_to_print: String,
}

#[derive(Clone, Debug)]
pub enum HalloyAction {
    Print(String),
    Command(String),
    Hook(Vec<HalloyHook>),
}

#[allow(dead_code)]
pub struct RustpythonExec {
    pub cmd: String,
    pub interp: Option<Rc<Interpreter>>,
    pub scope: Option<Scope>,
    pub clear_actions: bool,
}

#[allow(dead_code)]
pub struct RustpythonResult {
    pub out: String,
    pub interp: Rc<Interpreter>,
    pub error: Option<String>,
    pub scope: Scope,
    pub actions: Vec<Option<HalloyAction>>,
}

pub fn get_actions() -> Vec<Option<HalloyAction>> {
    let mut action: Vec<Option<HalloyAction>> = Vec::new();
    ACTIONS.with(|halloy_actions| {
        action = halloy_actions
            .lock()
            .expect("couldn't lock ACTIONS")
            .clone()
    });

    let act = action.clone();

    ACTIONS.with(|halloy_actions| {
        halloy_actions
            .lock()
            .expect("couldn't lock ACTIONS")
            .clear();
    });

    act
}

pub fn command_hooked(cmd: String) -> bool {
    let mut ret: bool = false;

    PY_HOOKS.with(|hooks| {
        let hooks = hooks.borrow();
        for (_, hook) in hooks.iter() {
            if let HalloyHook::ClientCommand(hook) = hook {
                if hook.command_name.contains(&cmd) {
                    ret = true;
                }
            }
        }
    });

    ret
}

pub fn push_command(cmd: Command) {
    PY_COMMAND_QUEUE.with(|queue| {
        let mut queue = queue.borrow_mut();

        queue.push(cmd)
    })
}

pub fn list_commands() -> Vec<Command> {
    let mut cmds: Vec<Command> = Vec::new();
    PY_COMMAND_QUEUE.with(|queue| {
        let mut queue = queue.borrow_mut();

        for cmd in queue.iter() {
            cmds.push(cmd.clone());
        }

        queue.clear();
    });

    cmds
}

pub fn set_pluginpref(key: String, value: String) {
    PY_PLUGINPREFS.with(|prefs| {
        let mut prefs = prefs.borrow_mut();

        prefs.insert(key, value);
    })
}

pub fn get_pluginpref(key: String) -> Option<String> {
    let mut result: Option<String> = None;

    PY_PLUGINPREFS.with(|prefs| {
        let prefs = prefs.borrow();

        match prefs.get(&key) {
            Some(pref) => result = Some(pref.clone()),

            _ => {}
        }
    });

    result
}

pub fn del_pluginpref(key: String) -> bool {
    let mut result: bool = true;

    PY_PLUGINPREFS.with(|prefs| {
        let mut prefs = prefs.borrow_mut();

        match prefs.remove(&key) {
            Some(_) => result = true,
            None => result = false,
        }
    });

    result
}

pub fn print_to_log(msg: String) {
    PY_PRINT.with(|pyprint| {
        let mut pyprint = pyprint.borrow_mut();
        pyprint.push(msg)
    })
}

pub fn print_queue(
    buffer: &buffer::Upstream,
    history: &mut history::Manager,
    config: &Config,
) {
    PY_PRINT.with(|pyprint| {
        let mut pyprint = pyprint.borrow_mut();
        for string in pyprint.iter() {
            let user = User::try_from("python-log!p@rustpython").unwrap();
            let buffer = Upstream::Query(
                buffer.server().clone(),
                Query::from_user(&user, CaseMap::ASCII),
            );
            let input_py = input::Input::plain(buffer.clone(), string.clone());
            history.record_input_message(
                input_py.clone(),
                user.clone(),
                &[user.clone()],
                &['O'], // placeholder
                &['O'], // placeholder
                CaseMap::ASCII,
                &config,
            );
        }

        pyprint.clear();
    })
}

pub fn append_to_hooks(hook: HalloyHook) -> u32 {
    let id = rand::rng().next_u32();
    PY_HOOKS.with(|hooks| {
        let mut hooks = hooks.borrow_mut();
        hooks.insert(id.clone(), hook);
    });

    id
}

pub fn run_hook(
    buffer: Option<&crate::buffer::Upstream>,
    hook_to_run: String,
    words: Vec<String>,
    command: bool,
    client_command: bool,
) {
    let mut py_functs: Vec<HalloyHook> = Vec::new();

    log::debug!("py: running hook {hook_to_run}");

    PY_HOOKS.with(|hooks| {
        let hooks = hooks.borrow();

        for (_id, hook) in hooks.iter() {
            py_functs.push(hook.clone());
        }
    });

    for py_funct in py_functs {
        match py_funct {
            HalloyHook::Print(hook) => {
                if hook.run_if != hook_to_run {
                    continue;
                }

                if command || client_command {
                    continue;
                }

                let py_funct = hook.py_funct;
                let result =
                    exec_funct(py_funct, Some(words.clone()), hook.attrs);

                print_to_log(result.out.clone());
                for action_ in result.actions.clone() {
                    if let Some(action) = action_ {
                        match action {
                            HalloyAction::Hook(hooks) => {
                                for hook in hooks {
                                    append_to_hooks(hook);
                                }
                            }

                            HalloyAction::Command(cmd) => {
                                match crate::command::parse(
                                    &cmd,
                                    buffer,
                                    &HashMap::new(),
                                ) {
                                    Ok(parsed_cmd) => {
                                        push_command(parsed_cmd.clone());
                                    }

                                    Err(_) => {
                                        log::debug!(
                                            "py: user passed invalid command in python!"
                                        );
                                    }
                                }
                            }

                            HalloyAction::Print(str) => {
                                print_to_log(str);
                            }
                        }
                    }
                }
            }

            HalloyHook::ClientCommand(hook) => {
                if !client_command {
                    continue;
                }

                if !(hook_to_run.contains(&hook.command_name)
                    || hook.command_name == "".to_owned())
                {
                    continue;
                }

                let py_funct = hook.py_funct;
                let result = exec_funct(py_funct, Some(words.clone()), None);

                print_to_log(result.out.clone());
                for action_ in result.actions.clone() {
                    if let Some(action) = action_ {
                        match action {
                            HalloyAction::Hook(hooks) => {
                                for hook in hooks {
                                    append_to_hooks(hook);
                                }
                            }

                            HalloyAction::Command(cmd) => {
                                match crate::command::parse(
                                    &cmd,
                                    buffer,
                                    &HashMap::new(),
                                ) {
                                    Ok(parsed_cmd) => {
                                        push_command(parsed_cmd);
                                    }

                                    Err(_) => {
                                        log::debug!(
                                            "py: user passed invalid command in python!"
                                        );
                                    }
                                }
                            }

                            HalloyAction::Print(str) => {
                                print_to_log(str);
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn exec_funct(
    py_funct: PyObjectRef,
    words: Option<Vec<String>>,
    attrs: Option<PyObjectRef>,
) -> RustpythonResult {
    let interp = Rc::new(Interpreter::with_init(Default::default(), |vm| {
        vm.add_native_modules(rustpython_stdlib::get_module_inits());
        vm.add_frozen(rustpython_pylib::FROZEN_STDLIB);

        vm.add_native_module(
            "hexchat".to_owned(),
            Box::new(hexchat_embedded::make_module.clone()),
        );
        vm.add_native_module(
            "xchat".to_owned(),
            Box::new(hexchat_embedded::make_module.clone()),
        );
    }));
    let mut err: Option<String> = None;
    let scope = interp.enter(|vm| {
        return vm.new_scope_with_builtins();
    });

    let mut args: Vec<PyObjectRef> = Vec::new();
    let mut args_eol: Vec<PyObjectRef> = Vec::new();

    let result: (String, Scope) = interp.enter(|vm| {
        let scope = scope.clone();

        if let Some(wrds) = words {
            if wrds.len() == 1 {
                for word in wrds.clone()[0].split(" ").collect_vec() {
                    args.push(word.to_pyobject(vm))
                }

                for i in 0..wrds.clone()[0].split(" ").collect_vec().len() {
                    args_eol.push(
                        wrds.clone()[0].split(" ").collect_vec()[i..]
                            .join(" ")
                            .to_pyobject(vm),
                    );
                }
            } else {
                for word in wrds.clone() {
                    args.push(word.clone().to_pyobject(vm))
                }

                for i in 0..wrds.clone().len() {
                    args_eol.push(wrds[i..].join(" ").to_pyobject(vm));
                }
            }
        }

        let _ = vm.import("sys", 0).unwrap();
        let io = vm.import("io", 0).unwrap();
        let string_io_type = io.get_attr("StringIO", vm).unwrap();
        let stdout = string_io_type.call((), vm).unwrap();
        vm.sys_module
            .set_attr("stdout", stdout.clone(), vm)
            .unwrap();
        vm.sys_module
            .set_attr("stderr", stdout.clone(), vm)
            .unwrap();

        let obj = py_funct.clone();
        let call;
        if let Some(attrs) = attrs {
            call = obj.call(
                (
                    vm.ctx.new_list(args),
                    vm.ctx.new_list(args_eol),
                    vm.ctx.new_list(Vec::new()),
                    attrs,
                ),
                vm,
            )
        } else {
            call = obj.call(
                (
                    vm.ctx.new_list(args),
                    vm.ctx.new_list(args_eol),
                    vm.ctx.new_list(Vec::new()),
                ),
                vm,
            )
        }

        match call {
            Ok(_) => {}
            Err(e) => {
                let mut buffer = String::new();
                let _ = vm.write_exception(&mut buffer, &e);
                err = Some(buffer.clone());
                return (String::new(), scope.clone());
            }
        }

        let output: PyStrRef = vm
            .call_method(&stdout, "getvalue", ())
            .unwrap()
            .try_into_value(vm)
            .unwrap();

        return (output.to_string(), scope.clone());
    });

    let mut action: Vec<Option<HalloyAction>> = Vec::new();
    ACTIONS.with(|halloy_actions| {
        action = halloy_actions
            .lock()
            .expect("couldn't lock ACTIONS")
            .clone()
    });

    let act = action.clone();

    ACTIONS.with(|halloy_actions| {
        halloy_actions
            .lock()
            .expect("couldn't lock ACTIONS")
            .clear();
    });

    return RustpythonResult {
        out: result.0,
        scope: result.1,
        error: err,
        interp: interp.clone(),
        actions: act.clone(),
    };
}

pub fn exec(rpexec: RustpythonExec) -> RustpythonResult {
    let interp = match rpexec.interp {
        Some(intr) => {
            log::debug!("reusing interpreter");
            intr
        }
        None => Rc::new(Interpreter::with_init(Default::default(), |vm| {
            vm.add_native_modules(rustpython_stdlib::get_module_inits());
            vm.add_frozen(rustpython_pylib::FROZEN_STDLIB);

            vm.add_native_module(
                "halloy".to_owned(),
                Box::new(hexchat_embedded::make_module.clone()),
            );
            vm.add_native_module(
                "hexchat".to_owned(),
                Box::new(hexchat_embedded::make_module.clone()),
            );
            vm.add_native_module(
                "xchat".to_owned(),
                Box::new(hexchat_embedded::make_module.clone()),
            );
        })),
    };
    let mut err: Option<String> = None;
    let scope = match rpexec.scope {
        Some(scp) => {
            log::debug!("reusing scope");
            scp
        }
        _ => interp.enter(|vm| {
            return vm.new_scope_with_builtins();
        }),
    };

    let result: (String, Scope) = interp.enter(|vm| {
        let scope = scope.clone();

        let _ = vm.import("sys", 0).unwrap();
        let io = vm.import("io", 0).unwrap();
        let string_io_type = io.get_attr("StringIO", vm).unwrap();
        let stdout = string_io_type.call((), vm).unwrap();
        vm.sys_module
            .set_attr("stdout", stdout.clone(), vm)
            .unwrap();
        vm.sys_module
            .set_attr("stderr", stdout.clone(), vm)
            .unwrap();

        let source = &rpexec.cmd;
        let code_obj = match vm
            .compile(
                source,
                rpvm::compiler::Mode::Exec,
                "<embedded>".to_owned(),
            )
            .map_err(|err| vm.new_syntax_error(&err, Some(source)))
        {
            Ok(obj) => obj,
            Err(e) => {
                let mut buffer = String::new();
                let _ = vm.write_exception(&mut buffer, &e);
                err = Some(buffer.clone());
                return (String::new(), scope.clone());
            }
        };
        match vm.run_code_obj(code_obj, scope.clone()) {
            Ok(_) => {}
            Err(e) => {
                let mut buffer = String::new();
                let _ = vm.write_exception(&mut buffer, &e);
                err = Some(buffer.clone());
                return (String::new(), scope.clone());
            }
        };

        let output: PyStrRef = vm
            .call_method(&stdout, "getvalue", ())
            .unwrap()
            .try_into_value(vm)
            .unwrap();

        return (output.to_string(), scope.clone());
    });

    let mut action: Vec<Option<HalloyAction>> = Vec::new();
    ACTIONS.with(|halloy_actions| {
        action = halloy_actions
            .lock()
            .expect("couldn't lock ACTIONS")
            .clone()
    });

    let act = action.clone();

    if rpexec.clear_actions {
        ACTIONS.with(|halloy_actions| {
            halloy_actions
                .lock()
                .expect("couldn't lock ACTIONS")
                .clear();
        });
    }

    return RustpythonResult {
        out: result.0,
        scope: result.1,
        error: err,
        interp: interp.clone(),
        actions: act.clone(),
    };
}

#[pymodule]
mod hexchat_embedded {
    use rustpython_vm::{PyObjectRef, PyResult, VirtualMachine};

    use super::{
        ACTIONS, HalloyAction, HalloyHook, RustpythonClientCommand,
        append_to_hooks, del_pluginpref as del_pref,
        get_pluginpref as get_pref, set_pluginpref as set_pref,
    };

    // print a string to the >>python<< buffer, or, if there's no buffer, to stdout
    #[pyfunction]
    fn prnt(value: PyObjectRef, vm: &VirtualMachine) {
        ACTIONS.with(|halloy_actions| {
            halloy_actions
                .lock()
                .expect("could not lock ACTIONS")
                .push(Some(HalloyAction::Print(
                    value.str(vm).unwrap().to_string(),
                )))
        });
    }

    #[pyfunction]
    fn command(cmd: String) {
        let command = if cmd.starts_with("/") {
            cmd
        } else {
            format!("/{cmd}")
        };
        ACTIONS.with(|halloy_actions| {
            halloy_actions
                .lock()
                .expect("could not lock ACTIONS")
                .push(Some(HalloyAction::Command(command)))
        })
    }

    // compare two strings (e.g. nicknames)
    #[pyfunction]
    fn nickcmp(s1: PyObjectRef, s2: PyObjectRef, vm: &VirtualMachine) -> u32 {
        return match s1.str(vm).unwrap().to_string()
            == s2.str(vm).unwrap().to_string()
        {
            true => 0,
            false => 1,
        }; // TODO: do better comparison?
    }

    // strip non-ascii chars from the string
    #[pyfunction]
    fn strip(value: PyObjectRef, vm: &VirtualMachine) -> String {
        let mut result = String::new();
        for char in value.str(vm).unwrap().to_string().chars() {
            if !(char.to_string().contains("\003")
                || char.to_string().contains("\002")
                || char.to_string().contains("\010")
                || char.to_string().contains("\037")
                || char.to_string().contains("\017")
                || char.to_string().contains("\026")
                || char.to_string().contains("\007")
                || char.to_string().contains("\035")
                || char.to_string().contains("\036"))
            {
                result += &char.to_string();
            }
        }

        result
    }

    #[pyfunction]
    fn hook_print(when_to_run: String, funct: PyObjectRef) -> u32 {
        append_to_hooks(HalloyHook::Print(super::RustpythonHookPrint {
            run_if: when_to_run,
            py_funct: funct,
            attrs: None,
        }))
    }

    #[pyfunction]
    fn hook_print_attrs(
        when_to_run: String,
        funct: PyObjectRef,
        attrs: Option<PyObjectRef>,
    ) -> u32 {
        append_to_hooks(HalloyHook::Print(super::RustpythonHookPrint {
            run_if: when_to_run,
            py_funct: funct,
            attrs: attrs.clone(),
        }))
    }

    #[pyfunction]
    fn hook_command(when_to_run: String, funct: PyObjectRef) -> u32 {
        append_to_hooks(HalloyHook::ClientCommand(RustpythonClientCommand {
            py_funct: funct.clone(),
            command_name: when_to_run.clone(),
        }))
    }

    #[pyfunction]
    fn hook_unload(_: PyObjectRef) {
        // we don't support unloading yet
    }

    #[pyfunction]
    fn get_pluginpref(
        name: String,
        vm: &VirtualMachine,
    ) -> PyResult<PyObjectRef> {
        match get_pref(name) {
            Some(value) => match value.clone().parse::<u32>() {
                Ok(int) => return Ok(vm.ctx.new_int(int).into()),

                _ => return Ok(vm.ctx.new_str(value.clone()).into()),
            },

            _ => return Ok(vm.ctx.none().into()),
        }
    }

    #[pyfunction]
    fn set_pluginpref(
        key: String,
        value: PyObjectRef,
        vm: &VirtualMachine,
    ) -> bool {
        set_pref(key, value.str(vm).unwrap().to_string());

        true
    }

    #[pyfunction]
    fn del_pluginpref(key: String) -> bool {
        del_pref(key)
    }
}
