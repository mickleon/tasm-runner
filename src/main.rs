extern crate getopts;
use getopts::Options;
use std::{env, process};
use tasm_runner::{do_work, get_args};

const USING: &str = "Использование: tasm-runner ФАЙЛ1 [ФАЙЛ2]... [параметры]";

const DESCRIPTION: &str =
    "Запускает компилятор и компоновщик TASM в Dosbox для создания исполняемого
файла из кода в файле(ов) ФАЙЛ1, ФАЙЛ2 .... В директории исходных файлов
появляется поддиректория \"/BUILD\" с исполняемым файлом";

fn print_usage(opts: Options) {
    let brief = format!("{USING}\n\n{DESCRIPTION}");
    println!("{}", opts.usage(&brief));
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut opts = Options::new();

    // Устанавливаем параметры и флаги
    opts.optopt(
        "c",
        "",
        "Устанавливает директорию (не путь до файла) с компилятором и компоновщиком \
    TASM. В ней должны содержаться файлы \"TASM.exe\" и \"TLINK.exe\"",
        "ДИРЕКТОРИЯ",
    );
    opts.optopt(
        "",
        "copts",
        "Принимает строку, в которой \
    содержатся параметры для компилятора TASM.exe. Например: \"/l /t\". Подробнее: \
    \"C:\\TASM\"",
        "ФЛАГИ",
    );
    opts.optopt(
        "",
        "lopts",
        "Принимает строку, в которой \
    содержатся параметры для компоновщика TLINK.exe. Например: \"/x /l\". По умолчанию: \"/x\" \
    Подробнее: C:\\TLINK\"",
        "ФЛАГИ",
    );
    opts.optflag("d", "debug", "Запустить дебаггер после компиляции");
    opts.optflag("e", "exit", "Выйти из dosbox после выполнения");
    opts.optflag("h", "help", "Выводит эту информацию");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            eprintln!("{f}");
            process::exit(-1);
        }
    };

    // Аргументов нет
    if matches.free.is_empty() {
        print_usage(opts);
        process::exit(-1);
    }

    // Помощь
    if matches.opt_present("h") {
        print_usage(opts);
        process::exit(0);
    }

    let config = match get_args(&matches) {
        Ok(config) => config,
        Err(e) => {
            println!("{e}");
            process::exit(-1);
        }
    };

    // Запуск работы с полученными параметрами
    match do_work(config) {
        Ok(status) => match status.code() {
            Some(code) => process::exit(code),
            None => process::exit(-1),
        },
        Err(e) => {
            println!("{e}");
            process::exit(-1);
        }
    }
}
