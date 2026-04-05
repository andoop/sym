//! Stack interpreter for [`crate::bytecode::Program`].

use std::cmp::Ordering;
use std::io::Write;

use crate::bytecode::{format_value_line, Instr, Program, ValCmpOp};

#[derive(Debug)]
pub struct VmError {
    pub message: String,
}

use crate::interp::Value;

fn pop_int(stack: &mut Vec<Value>, msg: &str) -> Result<i64, VmError> {
    match stack.pop() {
        Some(Value::Int(n)) => Ok(n),
        _ => Err(VmError {
            message: msg.into(),
        }),
    }
}

fn pop_bool(stack: &mut Vec<Value>, msg: &str) -> Result<bool, VmError> {
    match stack.pop() {
        Some(Value::Bool(b)) => Ok(b),
        _ => Err(VmError {
            message: msg.into(),
        }),
    }
}

fn pop_string(stack: &mut Vec<Value>, msg: &str) -> Result<String, VmError> {
    match stack.pop() {
        Some(Value::String(s)) => Ok(s),
        _ => Err(VmError {
            message: msg.into(),
        }),
    }
}

fn cmp_ordering_values(l: &Value, r: &Value) -> Result<Ordering, VmError> {
    match (l, r) {
        (Value::Int(a), Value::Int(b)) => Ok(a.cmp(b)),
        // UTF-8 字节序与 Unicode 标量字典序一致；与树解释器 `str` 比较相同。
        (Value::String(a), Value::String(b)) => Ok(a.cmp(&b)),
        _ => Err(VmError {
            message: "VM: comparison expects two Int or two String".into(),
        }),
    }
}

pub fn run(prog: &Program) -> Result<Value, VmError> {
    let mut stack: Vec<Value> = Vec::with_capacity(32);
    let mut frames: Vec<(usize, usize, Vec<Value>)> = Vec::new();
    let mut chunk_idx = prog.main_idx;
    let mut pc: usize = 0;
    let mut locals = vec![Value::Unit; prog.chunks[chunk_idx].local_count];

    loop {
        let chunk = &prog.chunks[chunk_idx];
        let Some(instr) = chunk.code.get(pc) else {
            return Err(VmError {
                message: format!("VM: pc {pc} out of range in chunk {chunk_idx}"),
            });
        };
        match instr {
            Instr::PushInt(n) => {
                stack.push(Value::Int(*n));
                pc += 1;
            }
            Instr::PushBool(b) => {
                stack.push(Value::Bool(*b));
                pc += 1;
            }
            Instr::PushUnit => {
                stack.push(Value::Unit);
                pc += 1;
            }
            Instr::PushStr(idx) => {
                let s = chunk.strings.get(*idx).cloned().ok_or_else(|| VmError {
                    message: format!("VM: bad string pool index {idx}"),
                })?;
                stack.push(Value::String(s));
                pc += 1;
            }
            Instr::Pop => {
                stack.pop().ok_or_else(|| VmError {
                    message: "VM: stack underflow on Pop".into(),
                })?;
                pc += 1;
            }
            Instr::LoadLocal(i) => {
                let v = locals.get(*i as usize).cloned().ok_or_else(|| VmError {
                    message: format!("VM: bad local index {i}"),
                })?;
                stack.push(v);
                pc += 1;
            }
            Instr::StoreLocal(i) => {
                let v = stack.pop().ok_or_else(|| VmError {
                    message: "VM: stack underflow on StoreLocal".into(),
                })?;
                let slot = locals.get_mut(*i as usize).ok_or_else(|| VmError {
                    message: format!("VM: bad local index {i}"),
                })?;
                *slot = v;
                pc += 1;
            }
            Instr::AddI => {
                let r = pop_int(&mut stack, "VM: `+` expects Int")?;
                let l = pop_int(&mut stack, "VM: `+` expects Int")?;
                stack.push(Value::Int(l + r));
                pc += 1;
            }
            Instr::SubI => {
                let r = pop_int(&mut stack, "VM: `-` expects Int")?;
                let l = pop_int(&mut stack, "VM: `-` expects Int")?;
                stack.push(Value::Int(l - r));
                pc += 1;
            }
            Instr::MulI => {
                let r = pop_int(&mut stack, "VM: `*` expects Int")?;
                let l = pop_int(&mut stack, "VM: `*` expects Int")?;
                stack.push(Value::Int(l * r));
                pc += 1;
            }
            Instr::DivI => {
                let r = pop_int(&mut stack, "VM: `/` expects Int")?;
                let l = pop_int(&mut stack, "VM: `/` expects Int")?;
                if r == 0 {
                    return Err(VmError {
                        message: "VM: division by zero".into(),
                    });
                }
                stack.push(Value::Int(l / r));
                pc += 1;
            }
            Instr::ModI => {
                let r = pop_int(&mut stack, "VM: `%` expects Int")?;
                let l = pop_int(&mut stack, "VM: `%` expects Int")?;
                if r == 0 {
                    return Err(VmError {
                        message: "VM: modulo by zero".into(),
                    });
                }
                stack.push(Value::Int(l.rem_euclid(r)));
                pc += 1;
            }
            Instr::EqI => {
                let r = pop_int(&mut stack, "VM: `==` expects Int")?;
                let l = pop_int(&mut stack, "VM: `==` expects Int")?;
                stack.push(Value::Bool(l == r));
                pc += 1;
            }
            Instr::NeI => {
                let r = pop_int(&mut stack, "VM: `!=` expects Int")?;
                let l = pop_int(&mut stack, "VM: `!=` expects Int")?;
                stack.push(Value::Bool(l != r));
                pc += 1;
            }
            Instr::LtI => {
                let r = pop_int(&mut stack, "VM: `<` expects Int")?;
                let l = pop_int(&mut stack, "VM: `<` expects Int")?;
                stack.push(Value::Bool(l < r));
                pc += 1;
            }
            Instr::LeI => {
                let r = pop_int(&mut stack, "VM: `<=` expects Int")?;
                let l = pop_int(&mut stack, "VM: `<=` expects Int")?;
                stack.push(Value::Bool(l <= r));
                pc += 1;
            }
            Instr::GtI => {
                let r = pop_int(&mut stack, "VM: `>` expects Int")?;
                let l = pop_int(&mut stack, "VM: `>` expects Int")?;
                stack.push(Value::Bool(l > r));
                pc += 1;
            }
            Instr::GeI => {
                let r = pop_int(&mut stack, "VM: `>=` expects Int")?;
                let l = pop_int(&mut stack, "VM: `>=` expects Int")?;
                stack.push(Value::Bool(l >= r));
                pc += 1;
            }
            Instr::EqB => {
                let r = pop_bool(&mut stack, "VM: `==` expects Bool")?;
                let l = pop_bool(&mut stack, "VM: `==` expects Bool")?;
                stack.push(Value::Bool(l == r));
                pc += 1;
            }
            Instr::NeB => {
                let r = pop_bool(&mut stack, "VM: `!=` expects Bool")?;
                let l = pop_bool(&mut stack, "VM: `!=` expects Bool")?;
                stack.push(Value::Bool(l != r));
                pc += 1;
            }
            Instr::NotB => {
                let b = pop_bool(&mut stack, "VM: `!` expects Bool")?;
                stack.push(Value::Bool(!b));
                pc += 1;
            }
            Instr::NegI => {
                let n = pop_int(&mut stack, "VM: unary `-` expects Int")?;
                stack.push(Value::Int(-n));
                pc += 1;
            }
            Instr::Jump(target) => {
                pc = *target;
            }
            Instr::JumpIfFalse(target) => {
                let b = pop_bool(&mut stack, "VM: branch expects Bool")?;
                if !b {
                    pc = *target;
                } else {
                    pc += 1;
                }
            }
            Instr::JumpIfTrue(target) => {
                let b = pop_bool(&mut stack, "VM: branch expects Bool")?;
                if b {
                    pc = *target;
                } else {
                    pc += 1;
                }
            }
            Instr::PrintLn { stderr, argc } => {
                let n = *argc as usize;
                let mut args = Vec::with_capacity(n);
                for _ in 0..n {
                    args.push(stack.pop().ok_or_else(|| VmError {
                        message: "VM: stack underflow on PrintLn".into(),
                    })?);
                }
                args.reverse();
                let mut line = String::new();
                for (i, v) in args.iter().enumerate() {
                    if i > 0 {
                        line.push(' ');
                    }
                    line.push_str(&format_value_line(v));
                }
                line.push('\n');
                if *stderr {
                    std::io::stderr()
                        .write_all(line.as_bytes())
                        .map_err(|e| VmError {
                            message: format!("VM: eprintln io: {e}"),
                        })?;
                } else {
                    std::io::stdout()
                        .write_all(line.as_bytes())
                        .map_err(|e| VmError {
                            message: format!("VM: println io: {e}"),
                        })?;
                }
                pc += 1;
            }
            Instr::CompareVal(cmp) => {
                let r = stack.pop().ok_or_else(|| VmError {
                    message: "VM: stack underflow on CompareVal".into(),
                })?;
                let l = stack.pop().ok_or_else(|| VmError {
                    message: "VM: stack underflow on CompareVal".into(),
                })?;
                let b = match cmp {
                    ValCmpOp::Eq => l == r,
                    ValCmpOp::Ne => l != r,
                    _ => {
                        let ord = cmp_ordering_values(&l, &r)?;
                        match cmp {
                            ValCmpOp::Lt => ord == Ordering::Less,
                            ValCmpOp::Le => ord != Ordering::Greater,
                            ValCmpOp::Gt => ord == Ordering::Greater,
                            ValCmpOp::Ge => ord != Ordering::Less,
                            ValCmpOp::Eq | ValCmpOp::Ne => unreachable!(),
                        }
                    }
                };
                stack.push(Value::Bool(b));
                pc += 1;
            }
            Instr::ConcatStr => {
                let r = pop_string(&mut stack, "VM: `concat` expects String")?;
                let mut l = pop_string(&mut stack, "VM: `concat` expects String")?;
                l.push_str(&r);
                stack.push(Value::String(l));
                pc += 1;
            }
            Instr::IntToStr => {
                let n = pop_int(&mut stack, "VM: `string_from_int` expects Int")?;
                stack.push(Value::String(n.to_string()));
                pc += 1;
            }
            Instr::StrLen => {
                let s = pop_string(&mut stack, "VM: `strlen` expects String")?;
                stack.push(Value::Int(s.chars().count() as i64));
                pc += 1;
            }
            Instr::Exit => {
                let n = pop_int(&mut stack, "VM: `exit` expects Int")?;
                std::process::exit(n as i32);
            }
            Instr::Call { fn_idx, argc } => {
                let callee = &prog.chunks[*fn_idx];
                let argc_u = *argc as usize;
                if callee.local_count < argc_u {
                    return Err(VmError {
                        message: "VM: callee local_count < argc".into(),
                    });
                }
                let mut new_locals = Vec::with_capacity(callee.local_count);
                for _ in 0..argc_u {
                    new_locals.push(stack.pop().ok_or_else(|| VmError {
                        message: "VM: stack underflow on Call".into(),
                    })?);
                }
                new_locals.reverse();
                new_locals.resize(callee.local_count, Value::Unit);
                frames.push((chunk_idx, pc + 1, std::mem::take(&mut locals)));
                chunk_idx = *fn_idx;
                pc = 0;
                locals = new_locals;
            }
            Instr::Ret => {
                let v = stack.pop().ok_or_else(|| VmError {
                    message: "VM: stack underflow on Ret".into(),
                })?;
                if let Some((ret_chunk, ret_pc, ret_locals)) = frames.pop() {
                    chunk_idx = ret_chunk;
                    pc = ret_pc;
                    locals = ret_locals;
                    stack.push(v);
                } else {
                    return Ok(v);
                }
            }
        }
    }
}
