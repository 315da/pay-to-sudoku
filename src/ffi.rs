use std::mem;
use std::slice;
use libc::{c_char, uint8_t, uint32_t, int32_t, c_void};

#[repr(C)]
struct Keypair;

#[derive(Copy, Clone)]
pub struct Context {
    keypair: *const Keypair,
    n: usize
}

#[link(name = "mysnark")]
extern "C" {
    pub fn mysnark_init_public_params();
    fn gen_keypair(n: uint32_t, h: *mut c_void, cb: extern fn(*mut c_void, *const c_char, int32_t, *const c_char, int32_t));
    fn load_keypair(pk_s: *const c_char, pk_l: int32_t, vk_s: *const c_char, vk_l: int32_t)
        -> *const Keypair;
    fn gen_proof(keypair: *const Keypair, h: *mut c_void,
                 cb: extern fn(*mut c_void, uint32_t, *const uint8_t, *const c_char, int32_t), 
                 n: uint32_t, puzzle: *const uint8_t, solution: *const uint8_t,
                 key: *const uint8_t, h_of_key: *const uint8_t) -> bool;
    fn decrypt_solution(n: uint32_t, enc: *mut uint8_t, key: *const uint8_t);
}

extern "C" fn handle_proof_callback(cb: *mut c_void, n: uint32_t, encrypted_solution: *const uint8_t, proof: *const c_char, proof_len: int32_t)
{
    unsafe {
        let proof: &[i8] = mem::transmute(slice::from_raw_parts(proof, proof_len as usize));
        let enc_solution: &[u8] = mem::transmute(slice::from_raw_parts(encrypted_solution, (n*n*n*n) as usize));

        let closure: &mut &mut for<'a> FnMut(&'a [u8], &'a [i8]) = mem::transmute(cb);

        closure(enc_solution, proof);
    }
}

extern "C" fn handle_keypair_callback(cb: *mut c_void, pk_s: *const c_char, pk_l: int32_t, vk_s: *const c_char, vk_l: int32_t)
{
    unsafe {
        let pk: &[i8] = mem::transmute(slice::from_raw_parts(pk_s, pk_l as usize));
        let vk: &[i8] = mem::transmute(slice::from_raw_parts(vk_s, vk_l as usize));

        let closure: &mut &mut for<'a> FnMut(&'a [i8], &'a [i8]) = mem::transmute(cb);

        closure(pk, vk);
    }
}

pub fn generate_keypair<F: for<'a> FnMut(&'a [i8], &'a [i8])>(num: u32, mut f: F) {
    let mut cb: &mut for<'a> FnMut(&'a [i8], &'a [i8]) = &mut f;

    unsafe {
        gen_keypair(num, (&mut cb) as *mut _ as *mut c_void, handle_keypair_callback);
    }
}

pub fn get_context(pk: &[i8], vk: &[i8], n: usize) -> Context {
    let keypair = unsafe {
        load_keypair(&pk[0], pk.len() as i32, &vk[0], vk.len() as i32)
    };

    Context {
        keypair: keypair,
        n: n
    }
}

pub fn prove<F: for<'a> FnMut(&'a [u8], &'a [i8])>(ctx: Context, puzzle: &[u8], solution: &[u8], key: &[u8], h_of_key: &[u8], mut f: F) -> bool {
    let mut cb: &mut for<'a> FnMut(&'a [u8], &'a [i8]) = &mut f;

    let cells = ctx.n.pow(4);
    assert_eq!(puzzle.len(), cells);
    assert_eq!(solution.len(), cells);
    assert_eq!(key.len(), 32);
    assert_eq!(h_of_key.len(), 32);

    unsafe {
        gen_proof(ctx.keypair, (&mut cb) as *mut _ as *mut c_void, handle_proof_callback, ctx.n as u32, &puzzle[0], &solution[0], &key[0], &h_of_key[0])
    }
}

pub fn decrypt(ctx: Context, enc_solution: &mut [u8], key: &[u8])
{
    assert_eq!(ctx.n.pow(4), enc_solution.len());
    assert_eq!(key.len(), 32);
    unsafe { decrypt_solution(ctx.n as u32, &mut enc_solution[0], &key[0]); }
}