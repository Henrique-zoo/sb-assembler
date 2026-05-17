use std::{env, fs};

use sb_assembler::assembler::Assembler;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Uso: sb-assembler <input_file>");
        std::process::exit(1);
    }

    let file_name = &args[1];
    match file_name {
        _ if file_name.ends_with(".asm") => {
            let file_content = fs::read_to_string(file_name).unwrap_or_else(|err| {
                eprintln!("Erro ao ler o arquivo {}: {}", file_name, err);
                std::process::exit(1);
            });
            let assembler = Assembler::new(file_content.as_str());
            assembler.generate_preprocessed_file(remove_file_extension(file_name));
        }
        _ if file_name.ends_with(".pre") => {}
        _ => {
            eprintln!(
                "Tipo de arquivo não suportado. Use .asm para arquivos de entrada ou .pre para arquivos pré-processados."
            );
            std::process::exit(1);
        }
    }
}

fn remove_file_extension(file_name: &str) -> &str {
    if let Some((idx, _)) = file_name.char_indices().rev().nth(3) {
        &file_name[..idx]
    } else {
        file_name
    }
}
