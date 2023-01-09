// pub const HELP: &'static str = "\
// default commands (type help <topic>):
// =====================================
// add      exit     open     reload   restart  start    avail
// remove   shutdown status   update   log      quit     stop     version";

pub const HELP: &'static str = "\
default commands (type help <topic>):
=====================================
exit     open     reload   restart  start    shutdown
status   update   quit     stop     version";

pub const HELP_HELP: &'static str = "\
help \t\tPrint a list of available actions\nhelp <action>\tPrint help for <action>";

// pub const HELP_AVAIL: &'static str = "avail\t\t\tDisplay all configured processes";
pub const HELP_QUIT: &'static str = "quit\tExit the supervisor shell.";
pub const HELP_EXIT: &'static str = "exit\tExit the supervisor shell.";
pub const HELP_SHUTDOWN: &'static str = "shutdown\t\tShut the remote taskmasterd down.";

pub const HELP_VERSION: &'static str = "\
version\t\t\tShow the version of the remote taskmasterd process";

pub const HELP_UPDATE: &'static str = "\
update\t\t\tReload config and add/remove as necessary, and will restart affected programs";

// pub const HELP_ADD: &'static str = "\
// add <name:seq> [...]	Activates any updates in config for process/group";

// pub const HELP_REMOVE: &'static str = "\
// remove <name:seq> [...]	Removes process/group from active config";

pub const HELP_STATUS: &'static str = "\
status <name:seq>		Get status for a single process
status <name:seq> <name:seq>	Get status for multiple named processes
status				Get all process status info";

pub const HELP_STOP: &'static str = "\
stop <name:seq>			Stop a process
stop <name:seq> <name:seq>	Stop multiple processes or groups
stop all			Stop all processes";

pub const HELP_RESTART: &'static str = "\
restart <name:seq>		Restart a process
restart <name:seq> <name:seq>	Restart multiple processes or groups
restart all			Restart all processes
Note: restart does not update config files. For that, see update.";

pub const HELP_START: &'static str = "\
start <name:seq>		Start a process
start <name:seq> <name:seq>	Start multiple processes or groups
start all			Start all processes";

pub const HELP_OPEN: &'static str = "\
open <path> 	Connect to a remote taskmasterd process.
		(for UNIX domain socket, use /path/to/socket)";

pub const HELP_RELOAD: &'static str = "\
reload 		Restart the remote taskmasterd.";
