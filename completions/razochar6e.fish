# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_razochar6e_global_optspecs
	string join \n h/help V/version
end

function __fish_razochar6e_needs_command
	# Figure out if the current invocation already has a command.
	set -l cmd (commandline -opc)
	set -e cmd[1]
	argparse -s (__fish_razochar6e_global_optspecs) -- $cmd 2>/dev/null
	or return
	if set -q argv[1]
		# Also print the command, so this can be used to figure out what it is.
		echo $argv[1]
		return 1
	end
	return 0
end

function __fish_razochar6e_using_subcommand
	set -l cmd (__fish_razochar6e_needs_command)
	test -z "$cmd"
	and return 1
	contains -- $cmd[1] $argv
end

complete -c razochar6e -n "__fish_razochar6e_needs_command" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c razochar6e -n "__fish_razochar6e_needs_command" -s V -l version -d 'Print version'
complete -c razochar6e -n "__fish_razochar6e_needs_command" -f -a "probe" -d 'Detect OS, batteries, and available charge-limit backends'
complete -c razochar6e -n "__fish_razochar6e_needs_command" -f -a "doctor" -d 'Run health checks (exit 1 if issues found)'
complete -c razochar6e -n "__fish_razochar6e_needs_command" -f -a "apply" -d 'Apply thresholds from ~/.config/razochar6e/config.toml'
complete -c razochar6e -n "__fish_razochar6e_needs_command" -f -a "set" -d 'Apply charge thresholds (requires root / Admin / supported hardware)'
complete -c razochar6e -n "__fish_razochar6e_needs_command" -f -a "status" -d 'Show battery status and current thresholds when readable'
complete -c razochar6e -n "__fish_razochar6e_needs_command" -f -a "clear" -d 'Reset to full charging (start=0, end=100)'
complete -c razochar6e -n "__fish_razochar6e_needs_command" -f -a "config" -d 'Manage ~/.config/razochar6e/config.toml'
complete -c razochar6e -n "__fish_razochar6e_needs_command" -f -a "install-persist" -d 'Install boot/login persistence for thresholds'
complete -c razochar6e -n "__fish_razochar6e_needs_command" -f -a "uninstall-persist" -d 'Remove persistence unit/task'
complete -c razochar6e -n "__fish_razochar6e_needs_command" -f -a "completions" -d 'Generate shell completions'
complete -c razochar6e -n "__fish_razochar6e_needs_command" -f -a "wsl" -d 'WSL: control Windows host battery via PowerShell bridge'
complete -c razochar6e -n "__fish_razochar6e_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand probe" -l json
complete -c razochar6e -n "__fish_razochar6e_using_subcommand probe" -s h -l help -d 'Print help'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand doctor" -s h -l help -d 'Print help'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand apply" -l backend -r
complete -c razochar6e -n "__fish_razochar6e_using_subcommand apply" -s h -l help -d 'Print help'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand set" -l start -r
complete -c razochar6e -n "__fish_razochar6e_using_subcommand set" -l end -r
complete -c razochar6e -n "__fish_razochar6e_using_subcommand set" -l backend -r
complete -c razochar6e -n "__fish_razochar6e_using_subcommand set" -l save -d 'Save values to config file'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand set" -s h -l help -d 'Print help'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand status" -s h -l help -d 'Print help'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand clear" -l backend -r
complete -c razochar6e -n "__fish_razochar6e_using_subcommand clear" -s h -l help -d 'Print help'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand config; and not __fish_seen_subcommand_from init show set help" -s h -l help -d 'Print help'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand config; and not __fish_seen_subcommand_from init show set help" -f -a "init" -d 'Write default config.toml'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand config; and not __fish_seen_subcommand_from init show set help" -f -a "show" -d 'Print config path and contents'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand config; and not __fish_seen_subcommand_from init show set help" -f -a "set" -d 'Set start/end in config without applying to hardware'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand config; and not __fish_seen_subcommand_from init show set help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand config; and __fish_seen_subcommand_from init" -s h -l help -d 'Print help'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand config; and __fish_seen_subcommand_from show" -s h -l help -d 'Print help'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand config; and __fish_seen_subcommand_from set" -l start -r
complete -c razochar6e -n "__fish_razochar6e_using_subcommand config; and __fish_seen_subcommand_from set" -l end -r
complete -c razochar6e -n "__fish_razochar6e_using_subcommand config; and __fish_seen_subcommand_from set" -l backend -r
complete -c razochar6e -n "__fish_razochar6e_using_subcommand config; and __fish_seen_subcommand_from set" -s h -l help -d 'Print help'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand config; and __fish_seen_subcommand_from help" -f -a "init" -d 'Write default config.toml'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand config; and __fish_seen_subcommand_from help" -f -a "show" -d 'Print config path and contents'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand config; and __fish_seen_subcommand_from help" -f -a "set" -d 'Set start/end in config without applying to hardware'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand config; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand install-persist" -l start -r
complete -c razochar6e -n "__fish_razochar6e_using_subcommand install-persist" -l end -r
complete -c razochar6e -n "__fish_razochar6e_using_subcommand install-persist" -s h -l help -d 'Print help'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand uninstall-persist" -s h -l help -d 'Print help'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand completions" -s h -l help -d 'Print help'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand wsl; and not __fish_seen_subcommand_from probe status set help" -s h -l help -d 'Print help'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand wsl; and not __fish_seen_subcommand_from probe status set help" -f -a "probe"
complete -c razochar6e -n "__fish_razochar6e_using_subcommand wsl; and not __fish_seen_subcommand_from probe status set help" -f -a "status"
complete -c razochar6e -n "__fish_razochar6e_using_subcommand wsl; and not __fish_seen_subcommand_from probe status set help" -f -a "set"
complete -c razochar6e -n "__fish_razochar6e_using_subcommand wsl; and not __fish_seen_subcommand_from probe status set help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand wsl; and __fish_seen_subcommand_from probe" -s h -l help -d 'Print help'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand wsl; and __fish_seen_subcommand_from status" -s h -l help -d 'Print help'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand wsl; and __fish_seen_subcommand_from set" -l start -r
complete -c razochar6e -n "__fish_razochar6e_using_subcommand wsl; and __fish_seen_subcommand_from set" -l end -r
complete -c razochar6e -n "__fish_razochar6e_using_subcommand wsl; and __fish_seen_subcommand_from set" -s h -l help -d 'Print help'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand wsl; and __fish_seen_subcommand_from help" -f -a "probe"
complete -c razochar6e -n "__fish_razochar6e_using_subcommand wsl; and __fish_seen_subcommand_from help" -f -a "status"
complete -c razochar6e -n "__fish_razochar6e_using_subcommand wsl; and __fish_seen_subcommand_from help" -f -a "set"
complete -c razochar6e -n "__fish_razochar6e_using_subcommand wsl; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand help; and not __fish_seen_subcommand_from probe doctor apply set status clear config install-persist uninstall-persist completions wsl help" -f -a "probe" -d 'Detect OS, batteries, and available charge-limit backends'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand help; and not __fish_seen_subcommand_from probe doctor apply set status clear config install-persist uninstall-persist completions wsl help" -f -a "doctor" -d 'Run health checks (exit 1 if issues found)'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand help; and not __fish_seen_subcommand_from probe doctor apply set status clear config install-persist uninstall-persist completions wsl help" -f -a "apply" -d 'Apply thresholds from ~/.config/razochar6e/config.toml'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand help; and not __fish_seen_subcommand_from probe doctor apply set status clear config install-persist uninstall-persist completions wsl help" -f -a "set" -d 'Apply charge thresholds (requires root / Admin / supported hardware)'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand help; and not __fish_seen_subcommand_from probe doctor apply set status clear config install-persist uninstall-persist completions wsl help" -f -a "status" -d 'Show battery status and current thresholds when readable'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand help; and not __fish_seen_subcommand_from probe doctor apply set status clear config install-persist uninstall-persist completions wsl help" -f -a "clear" -d 'Reset to full charging (start=0, end=100)'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand help; and not __fish_seen_subcommand_from probe doctor apply set status clear config install-persist uninstall-persist completions wsl help" -f -a "config" -d 'Manage ~/.config/razochar6e/config.toml'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand help; and not __fish_seen_subcommand_from probe doctor apply set status clear config install-persist uninstall-persist completions wsl help" -f -a "install-persist" -d 'Install boot/login persistence for thresholds'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand help; and not __fish_seen_subcommand_from probe doctor apply set status clear config install-persist uninstall-persist completions wsl help" -f -a "uninstall-persist" -d 'Remove persistence unit/task'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand help; and not __fish_seen_subcommand_from probe doctor apply set status clear config install-persist uninstall-persist completions wsl help" -f -a "completions" -d 'Generate shell completions'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand help; and not __fish_seen_subcommand_from probe doctor apply set status clear config install-persist uninstall-persist completions wsl help" -f -a "wsl" -d 'WSL: control Windows host battery via PowerShell bridge'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand help; and not __fish_seen_subcommand_from probe doctor apply set status clear config install-persist uninstall-persist completions wsl help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand help; and __fish_seen_subcommand_from config" -f -a "init" -d 'Write default config.toml'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand help; and __fish_seen_subcommand_from config" -f -a "show" -d 'Print config path and contents'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand help; and __fish_seen_subcommand_from config" -f -a "set" -d 'Set start/end in config without applying to hardware'
complete -c razochar6e -n "__fish_razochar6e_using_subcommand help; and __fish_seen_subcommand_from wsl" -f -a "probe"
complete -c razochar6e -n "__fish_razochar6e_using_subcommand help; and __fish_seen_subcommand_from wsl" -f -a "status"
complete -c razochar6e -n "__fish_razochar6e_using_subcommand help; and __fish_seen_subcommand_from wsl" -f -a "set"
