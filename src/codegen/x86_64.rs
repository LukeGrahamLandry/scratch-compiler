use crate::{
    diagnostic::{Error, Result},
    ir::{expr::Expr, proc::Procedure, statement::Statement, Program},
    span::Span,
    uid::Uid,
};
use sb3_stuff::Value;
use std::{
    fmt::{self, Write as _},
    fs::File,
    io::Write as _,
    iter,
    path::Path,
};

pub fn write_asm_file(program: &Program, path: &Path) -> Result<()> {
    let mut asm_program = AsmProgram {
        uid_generator: crate::uid::Generator::new(),
        entry_points: Vec::new(),
        text: String::new(),
    };

    for (name, procs) in iter::once(&program.stage)
        .chain(program.sprites.values())
        .flat_map(|sprite| &sprite.procedures)
    {
        for proc in procs {
            asm_program.generate_proc(name, proc)?;
        }
    }

    let mut file = File::create(path).unwrap();
    write!(file, "{asm_program}").unwrap();

    Ok(())
}

struct AsmProgram {
    uid_generator: crate::uid::Generator,
    entry_points: Vec<Uid>,
    text: String,
}

impl AsmProgram {
    fn new_uid(&self) -> Uid {
        self.uid_generator.new_uid()
    }

    fn emit<T: Emit>(&mut self, t: T) {
        t.emit(self);
    }

    fn generate_proc(&mut self, name: &str, proc: &Procedure) -> Result<Uid> {
        match name {
            "when-flag-clicked" => {
                assert!(proc.params.is_empty());
                let proc_id = self.new_uid();
                self.entry_points.push(proc_id);
                self.emit(Label(proc_id));
                self.generate_statement(&proc.body)?;
                self.text.push_str("    ret\n");
                Ok(proc_id)
            }
            _ => todo!(),
        }
    }

    fn generate_statement(&mut self, stmt: &Statement) -> Result<()> {
        match stmt {
            Statement::ProcCall {
                proc_name, args, ..
            } => self.generate_proc_call(proc_name, args),
            Statement::Do(stmts) => stmts
                .iter()
                .try_for_each(|stmt| self.generate_statement(stmt)),
            Statement::IfElse { .. } => todo!(),
            Statement::Repeat { .. } => todo!(),
            Statement::Forever(body) => {
                let loop_label = self.new_uid();
                self.emit(Label(loop_label));
                self.generate_statement(body)?;
                writeln!(self.text, "    jmp {loop_label}").unwrap();
                Ok(())
            }
            Statement::Until { .. } => todo!(),
            Statement::While { .. } => todo!(),
            Statement::For { .. } => todo!(),
        }
    }

    fn generate_proc_call(
        &mut self,
        proc_name: &str,
        args: &[Expr],
    ) -> Result<()> {
        match proc_name {
            "print" => match args {
                [message] => {
                    if let Expr::Lit(message) = message {
                        let message = message.to_cow_str();
                        let message_id = self.allocate_static_str(&message);
                        writeln!(
                            self.text,
                            "    mov rax, 1
    mov rdi, 1
    mov rsi, {message_id}
    mov rdx, {}
    syscall",
                            message.len(),
                        )
                        .unwrap();
                    } else {
                        self.generate_expr(message)?;
                        self.cowify();
                        self.text.push_str(
                            "    mov rdx, rsi
    mov rsi, rdi
    mov rax, 1
    mov rdi, 1
    syscall
",
                        );
                        self.drop_pop();
                    }
                }
                _ => todo!(),
            },
            _ => todo!(),
        }
        Ok(())
    }

    fn generate_expr(&mut self, expr: &Expr) -> Result<()> {
        match expr {
            Expr::Lit(lit) => {
                self.push_lit(lit);
                Ok(())
            }
            Expr::Sym(_, _) => todo!(),
            Expr::FuncCall(func_name, span, args) => {
                self.generate_func_call(func_name, args, *span)
            }
            Expr::AddSub(_, _) => todo!(),
            Expr::MulDiv(_, _) => todo!(),
        }
    }

    fn generate_func_call(
        &mut self,
        func_name: &str,
        args: &[Expr],
        span: Span,
    ) -> Result<()> {
        match func_name {
            "!!" => todo!(),
            "++" => match args {
                [single] => self.generate_expr(single),
                [lhs, rhs] => {
                    self.text.push_str("    push 0\n    push 0\n");
                    self.generate_expr(rhs)?;
                    self.cowify();
                    self.generate_expr(lhs)?;
                    self.cowify();
                    self.text.push_str(
                        "    mov rdi, [rsp+24]
    add rdi, rsi
    mov [rsp+40], rdi
    call malloc
    mov [rsp+32], rax
    mov rdi, rax
    mov rdx, rsi
    mov rsi, [rsp]
    call memcpy
    mov rdi, rax
    add rdi, [rsp+8]
    mov rsi, [rsp+16]
    mov rdx, [rsp+24]
    call memcpy
",
                    );

                    self.drop_pop();
                    self.drop_pop();
                    Ok(())
                }
                _ => todo!(),
            },
            "and" => todo!(),
            "or" => todo!(),
            "not" => todo!(),
            "=" => todo!(),
            "<" => todo!(),
            ">" => todo!(),
            "length" => todo!(),
            "str-length" => todo!(),
            "char-at" => {
                todo!()
            }
            "mod" => todo!(),
            "abs" => todo!(),
            "floor" => todo!(),
            "ceil" => todo!(),
            "sqrt" => todo!(),
            "ln" => todo!(),
            "log" => todo!(),
            "e^" => todo!(),
            "ten^" => todo!(),
            "sin" => todo!(),
            "cos" => todo!(),
            "tan" => todo!(),
            "asin" => todo!(),
            "acos" => todo!(),
            "atan" => todo!(),
            "pressing-key" => todo!(),
            "to-num" => todo!(),
            "random" => todo!(),
            _ => Err(Box::new(Error::UnknownFunction {
                span,
                func_name: func_name.to_owned(),
            })),
        }
    }

    fn allocate_static_str(&mut self, s: &str) -> Uid {
        let uid = self.new_uid();
        write!(self.text, "staticstr {uid}, db ").unwrap();
        for (i, byte) in s.bytes().enumerate() {
            if i == 0 {
                write!(self.text, "{byte}").unwrap();
            } else {
                write!(self.text, ",{byte}").unwrap();
            }
        }
        self.text.push('\n');
        uid
    }

    fn push_lit(&mut self, lit: &Value) {
        match lit {
            Value::Num(num) => {
                let bits = num.to_bits();
                writeln!(
                    self.text,
                    "    mov rax, {bits}
    push rax
    push 2",
                )
                .unwrap();
            }
            Value::String(s) => {
                let string_id = self.allocate_static_str(s);
                let len = s.len();
                writeln!(
                    self.text,
                    "    mov rax, {len}
    push rax
    mov rcx, {string_id}
    push rcx",
                )
                .unwrap();
            }
            Value::Bool(false) => {
                writeln!(
                    self.text,
                    "    push 0
    push 0",
                )
                .unwrap();
            }
            Value::Bool(true) => {
                writeln!(
                    self.text,
                    "    push 0
    push 1",
                )
                .unwrap();
            }
        }
    }

    fn cowify(&mut self) {
        self.text.push_str("    call cowify\n");
    }

    fn drop_pop(&mut self) {
        self.text.push_str("    call drop_pop\n");
    }
}

impl fmt::Display for AsmProgram {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, concat!(include_str!("x86_64/prelude.s"), "\nmain:\n"))?;
        for entry_point in &self.entry_points {
            writeln!(f, "    call {entry_point}")?;
        }
        write!(
            f,
            r#"    mov rax, 60
    mov rdi, 0
    syscall

{}"#,
            self.text,
        )?;
        Ok(())
    }
}

trait Emit {
    fn emit(self, program: &mut AsmProgram);
}

struct Label<T>(T);

impl<T: fmt::Display> Emit for Label<T> {
    fn emit(self, program: &mut AsmProgram) {
        writeln!(program.text, "{}:", self.0).unwrap();
    }
}
