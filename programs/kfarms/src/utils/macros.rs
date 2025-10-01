#[macro_export]
macro_rules! gen_signer_seeds {
    (
    $seed: expr, $first_key: expr, $second_key: expr, $bump: expr
) => {
        &[&[$seed, $first_key.as_ref(), $second_key.as_ref(), &[$bump]]]
    };
}

#[macro_export]
macro_rules! gen_signer_seeds_two {
    (
    $seed: expr, $first_key: expr, $bump: expr
) => {
        &[&[$seed, $first_key.as_ref(), &[$bump]]]
    };
}

#[cfg(target_arch = "bpf")]
#[macro_export]
macro_rules! xmsg {
    ($($arg:tt)*) => {{
        ::anchor_lang::solana_program::log::sol_log(&format!($($arg)*));
    }};
}

#[cfg(not(target_arch = "bpf"))]
#[macro_export]
macro_rules! xmsg {
    ($($arg:tt)*) => {{
        println!($($arg)*);
    }};
}

#[cfg(target_arch = "bpf")]
#[macro_export]
macro_rules! dbg_msg {
   
   
   
   
    () => {
        msg!("[{}:{}]", file!(), line!())
    };
    ($val:expr $(,)?) => {
       
       
        match $val {
            tmp => {
                msg!("[{}:{}] {} = {:#?}",
                    file!(), line!(), stringify!($val), &tmp);
                tmp
            }
        }
    };
    ($($val:expr),+ $(,)?) => {
        ($($crate::dbg_msg!($val)),+,)
    };
}

#[cfg(not(target_arch = "bpf"))]
#[macro_export]
macro_rules! dbg_msg {
   
   
   
   
    () => {
        println!("[{}:{}]", file!(), line!())
    };
    ($val:expr $(,)?) => {
       
       
        match $val {
            tmp => {
                println!("[{}:{}] {} = {:#?}",
                    file!(), line!(), stringify!($val), &tmp);
                tmp
            }
        }
    };
    ($($val:expr),+ $(,)?) => {
        ($($crate::dbg_msg!($val)),+,)
    };
}
