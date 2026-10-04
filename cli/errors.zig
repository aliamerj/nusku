//! Error vocabulary shared by commands and `execute`.
//!
//! A command's `run` can return any error. Two names have special meaning:
//!
//!   error.Reported      the command already printed its message (ctx.fail)
//!   error.InvalidUsage  bad input; exit 2 and show a help hint (ctx.usageFail)
//!
//! Anything else is printed as "prog: error: <name>" and exits 1.

pub const RunError = error{ Reported, InvalidUsage };

pub const exit_success: u8 = 0;
pub const exit_failure: u8 = 1;
pub const exit_usage: u8 = 2;
