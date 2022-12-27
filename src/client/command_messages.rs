pub const HELP: &'static str = "\
default commands (type help <topic>):
=====================================
add      exit     open     reload   restart  start    avail
remove   shutdown status   update   log      quit     stop     version";

pub const HELP_HELP: &'static str = "\
help \t\tPrint a list of available actions\nhelp <action>\tPrint help for <action>";

pub const HELP_AVAIL: &'static str = "avail\t\t\tDisplay all configured processes";
pub const HELP_QUIT: &'static str = "quit\tExit the supervisor shell.";
pub const HELP_EXIT: &'static str = "exit\tExit the supervisor shell.";
pub const HELP_SHUTDOWN: &'static str = "shutdown\t\tShut the remote supervisord down.";

pub const HELP_VERSION: &'static str = "\
version\t\t\tShow the version of the remote supervisord process";

pub const HELP_UPDATE: &'static str = "\
update\t\t\tReload config and add/remove as necessary, and will restart affected programs";

pub const HELP_ADD: &'static str = "\
add <name> [...]	Activates any updates in config for process/group";

pub const HELP_REMOVE: &'static str = "\
remove <name> [...]	Removes process/group from active config";

pub const HELP_STATUS: &'static str = "\
status <name>		Get status for a single process
status <name> <name>	Get status for multiple named processes
status			Get all process status info";

pub const HELP_STOP: &'static str = "\
stop <name>		Stop a process
stop <name> <name>	Stop multiple processes or groups
stop all		Stop all processes";

pub const HELP_RESTART: &'static str = "\
restart <name>		Restart a process
restart <name> <name>	Restart multiple processes or groups
restart all		Restart all processes
Note: restart does not reread config files. For that, see reread and update.";

pub const HELP_START: &'static str = "\
start <name>		Start a process
start <name> <name>	Start multiple processes or groups
start all		Start all processes";

pub const HELP_OPEN: &'static str = "\
open <path> 	Connect to a remote supervisord process.
		(for UNIX domain socket, use /path/to/socket)";

pub const HELP_RELOAD: &'static str = "\
reload 		Restart the remote supervisord.";
