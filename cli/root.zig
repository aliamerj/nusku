//! Public API of the cli module.

const command = @import("command.zig");
const parser = @import("parser.zig");
const help = @import("help.zig");
const execute_mod = @import("execute.zig");

pub const Command = command.Command;
pub const Flag = command.Flag;
pub const FlagKind = command.FlagKind;
pub const ArgsRule = command.ArgsRule;
pub const Context = command.Context;
pub const FlagValues = command.FlagValues;
pub const RunFn = command.RunFn;

pub const parse = parser.parse;
pub const ParseError = parser.Error;
pub const Diagnostic = parser.Diagnostic;
pub const ParseOptions = parser.Options;
pub const Parsed = parser.Parsed;
pub const EnvLookup = parser.EnvLookup;

pub const execute = execute_mod.execute;
pub const ExecuteOptions = execute_mod.Options;
