//! Interface to the pluggable authentication module framework (PAM).
//!
//! The goal of this library is to provide a type-safe API that can be used to
//! interact with PAM.  The library is incomplete - currently it supports
//! a subset of functions for use in a pam authentication module.  A pam module
//! is a shared library that is invoked to authenticate a user, or to perform
//! other functions.
//!
//! For general information on writing pam modules, see
//! [The Linux-PAM Module Writers' Guide][module-guide]
//!
//! [module-guide]: http://www.linux-pam.org/Linux-PAM-html/Linux-PAM_MWG.html
//!
//! A typical authentication module will define an external function called
//! `pam_sm_authenticate()`, which will use functions in this library to
//! interrogate the program that requested authentication for more information,
//! and to render a result.  For a working example that uses this library, see
//! [toznyauth-pam][].
//!
//! [toznyauth-pam]: https://github.com/tozny/toznyauth-pam
//!
//! Note that constants that are normally read from pam header files are
//! hard-coded in the `constants` module.  The values there are taken from
//! a Linux system.  That means that it might take some work to get this library
//! to work on other platforms.

pub mod constants;
pub mod conv;
pub mod items;
#[doc(hidden)]
pub mod macros;
pub mod module;

use std::collections::BTreeSet;
use std::convert::TryFrom;
use std::ffi::CStr;

use crate::pam::constants::*;
use crate::pam::conv::PamConv;
use crate::pam::module::{PamHandle, PamHooks};
use crate::pam_hooks;
use constants::PamResultCode;

#[derive(Debug)]
struct Options {
    debug: bool,
    use_first_pass: bool,
    ignore_unknown_user: bool,
}

impl TryFrom<&Vec<&CStr>> for Options {
    type Error = ();

    fn try_from(args: &Vec<&CStr>) -> Result<Self, Self::Error> {
        let opts: Result<BTreeSet<&str>, _> = args.iter().map(|cs| cs.to_str()).collect();
        let gopts = match opts {
            Ok(o) => o,
            Err(e) => {
                println!("Error in module args -> {:?}", e);
                return Err(());
            }
        };

        Ok(Options {
            debug: gopts.contains("debug"),
            use_first_pass: gopts.contains("use_first_pass"),
            ignore_unknown_user: gopts.contains("ignore_unknown_user"),
        })
    }
}

pub struct PamHimmelBlau;

pam_hooks!(PamHimmelBlau);

impl PamHooks for PamHimmelBlau {
    fn acct_mgmt(pamh: &PamHandle, args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
        let opts = match Options::try_from(&args) {
            Ok(o) => o,
            Err(_) => return PamResultCode::PAM_SERVICE_ERR,
        };

        let tty = pamh.get_tty();
        let rhost = pamh.get_rhost();

        if opts.debug {
            println!("acct_mgmt");
            println!("args -> {:?}", args);
            println!("opts -> {:?}", opts);
            println!("tty -> {:?} rhost -> {:?}", tty, rhost);
        }

        let account_id = match pamh.get_user(None) {
            Ok(aid) => aid,
            Err(e) => {
                if opts.debug {
                    println!("Error get_user -> {:?}", e);
                }
                return e;
            }
        };

        return PamResultCode::PAM_IGNORE;
    }

    fn sm_authenticate(pamh: &PamHandle, args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
        let opts = match Options::try_from(&args) {
            Ok(o) => o,
            Err(_) => return PamResultCode::PAM_SERVICE_ERR,
        };

        // This will == "Ok(Some("ssh"))" on remote auth.
        let tty = pamh.get_tty();
        let rhost = pamh.get_rhost();

        if opts.debug {
            println!("sm_authenticate");
            println!("args -> {:?}", args);
            println!("opts -> {:?}", opts);
            println!("tty -> {:?} rhost -> {:?}", tty, rhost);
        }

        let account_id = match pamh.get_user(None) {
            Ok(aid) => aid,
            Err(e) => {
                println!("Error get_user -> {:?}", e);
                return e;
            }
        };

        let authtok = match pamh.get_authtok() {
            Ok(atok) => atok,
            Err(e) => {
                if opts.debug {
                    println!("Error get_authtok -> {:?}", e);
                }
                return e;
            }
        };

        let authtok = match authtok {
            Some(v) => v,
            None => {
                if opts.use_first_pass {
                    if opts.debug {
                        println!("Don't have an authtok, returning PAM_AUTH_ERR");
                    }
                    return PamResultCode::PAM_AUTH_ERR;
                } else {
                    let conv = match pamh.get_item::<PamConv>() {
                        Ok(conv) => conv,
                        Err(err) => {
                            if opts.debug {
                                println!("Couldn't get pam_conv");
                            }
                            return err;
                        }
                    };
                    match conv.send(PAM_PROMPT_ECHO_OFF, "Password: ") {
                        Ok(password) => match password {
                            Some(pw) => pw,
                            None => {
                                if opts.debug {
                                    println!("No password");
                                }
                                return PamResultCode::PAM_CRED_INSUFFICIENT;
                            }
                        },
                        Err(err) => {
                            if opts.debug {
                                println!("Couldn't get password");
                            }
                            return err;
                        }
                    }
                } // end opts.use_first_pass
            }
        };

        return PamResultCode::PAM_IGNORE;
    }

    fn sm_chauthtok(_pamh: &PamHandle, args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
        let opts = match Options::try_from(&args) {
            Ok(o) => o,
            Err(_) => return PamResultCode::PAM_SERVICE_ERR,
        };

        if opts.debug {
            println!("sm_chauthtok");
            println!("args -> {:?}", args);
            println!("opts -> {:?}", opts);
        }
        PamResultCode::PAM_IGNORE
    }

    fn sm_close_session(_pamh: &PamHandle, args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
        let opts = match Options::try_from(&args) {
            Ok(o) => o,
            Err(_) => return PamResultCode::PAM_SERVICE_ERR,
        };

        if opts.debug {
            println!("sm_close_session");
            println!("args -> {:?}", args);
            println!("opts -> {:?}", opts);
        }
        PamResultCode::PAM_SUCCESS
    }

    fn sm_open_session(pamh: &PamHandle, args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
        let opts = match Options::try_from(&args) {
            Ok(o) => o,
            Err(_) => return PamResultCode::PAM_SERVICE_ERR,
        };

        if opts.debug {
            println!("sm_open_session");
            println!("args -> {:?}", args);
            println!("opts -> {:?}", opts);
        }

        let account_id = match pamh.get_user(None) {
            Ok(aid) => aid,
            Err(e) => {
                if opts.debug {
                    println!("Error get_user -> {:?}", e);
                }
                return e;
            }
        };

        return PamResultCode::PAM_IGNORE;
    }

    fn sm_setcred(_pamh: &PamHandle, args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
        let opts = match Options::try_from(&args) {
            Ok(o) => o,
            Err(_) => return PamResultCode::PAM_SERVICE_ERR,
        };

        if opts.debug {
            println!("sm_setcred");
            println!("args -> {:?}", args);
            println!("opts -> {:?}", opts);
        }
        PamResultCode::PAM_SUCCESS
    }
}
