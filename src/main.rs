extern crate getopts;
use std::{path::PathBuf, process::{Command, ExitStatus, Stdio}};
use getopts::{Matches, Options};
use std::{env, fs::exists, io, path, process};
use path::Path;

const USING: &str = "Использование: tasm-runner ФАЙЛ [параметры]";

const DESCRIPTION: &str = 
"Запускает компилятор и компоновщик TASM в Dosbox для создания исполняемого
файла из кода в файле ФАЙЛ";

fn print_usage(opts: Options) {
    let brief = format!("{USING}\n\n{DESCRIPTION}");
    println!("{}", opts.usage(&brief));
}

fn to_absolute_path(path_str: &str) -> Result<PathBuf, io::Error> {
    let path = Path::new(path_str);
    
    if path.is_absolute() {
        match path.canonicalize() {
            Ok(path) => Ok(path),
            Err(e) => return Err(
                io::Error::new(e.kind(),
                format!("{path:?}: {e}")
            ))
        }
    } else {
        let current_dir = env::current_dir()?;
        match current_dir.join(path).canonicalize() {
            Ok(path) => Ok(path),
            Err(e) => return Err(
                io::Error::new(e.kind(),
                format!("{path:?}: {e}")
            ))
        }
    }
}

// Обертка для exists() с боллее понятным выводом ошибки
fn check_file(path: PathBuf) -> Result<(), io::Error> {
    match exists(&path) {
        Ok(file_exists) => {
            if file_exists {
                Ok(())
            } else {
                Err(
                    io::Error::new(io::ErrorKind::NotFound,
                    format!("Файл {path:?} не найден")
                ))
            }
        },
        Err(e) => return Err(
            io::Error::new(e.kind(),
            format!("{path:?}: {}", e.to_string())
        ))
    }
}

// Возвращает путь до компилятора
fn get_compiler_dir(matches: &Matches) -> Result<PathBuf, io::Error> {
    let input = match matches.opt_str("c") {
        Some(path) => path,
        None => return Err(
            io::Error::new(io::ErrorKind::InvalidInput,
            "Путь до компилятора TASM не указан. Подробнее: --help"
        ))
    };

    match to_absolute_path(&input) {
        Ok(compiler_absolute) => {
            let mut tasm_path = compiler_absolute.clone();
            let mut tlink_path = compiler_absolute.clone();
            tasm_path.push("TASM.exe");
            tlink_path.push("TLINK.exe");

            match check_file(tasm_path) {
                Ok(_) => {},
                Err(e) => return Err(e)
            }

            match check_file(tlink_path) {
                Ok(_) => {},
                Err(e) => return Err(e)
            }

            Ok(compiler_absolute)
        },
        Err(e) => Err(e)
    }
}

// Заполняет вектор `commands` нужными командами для dosbox
fn generate_commands(file_path: PathBuf, compiler_dir: PathBuf, exit: bool, commands: &mut Vec<String>) {
    let mut file_dir = file_path.clone();
    file_dir.pop(); // Получаем директорию файла
    
    let binding = file_path.clone();

    let file_name = binding
        .file_name()
        .unwrap()
        .to_str()
        .unwrap(); // Имя компилируемого файла

    let file_stem = binding
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap(); // Имя компилируемого файла без расширения

    let mut build_dir = file_dir.clone();
    build_dir.push("BUILD/"); // Получаем директорию будущего исполняемого файла

    commands.push(format!("mount C: {compiler_dir:?}"));

    let working_drive = if file_dir == compiler_dir {
        "C:"
    } else {
        "D:"
    };

    if working_drive != "C:" {
        commands.push(format!("mount {working_drive} {file_dir:?}"));
    }

    commands.push(working_drive.to_string());   
    commands.push("md BUILD".to_string());
    commands.push("cd BUILD".to_string());
    commands.push("cls".to_string());
    commands.push(format!("C:\\TASM.EXE {working_drive}\\{file_name}"));
    commands.push(format!("C:\\TLINK.EXE {working_drive}\\BUILD\\{file_stem}.OBJ"));
    commands.push(format!("{working_drive}\\BUILD\\{file_stem}.exe"));

    if exit {
        commands.push("@pause".to_string());
        commands.push("exit 0".to_string());
    }
}

// Запуск работы
fn do_work(file_path: PathBuf, compiler_dir: PathBuf, exit: bool) -> Result<ExitStatus, std::io::Error> {
    let mut commands = Vec::new(); // Вектор команд для выполнения в dosbox
    generate_commands(file_path, compiler_dir, exit, &mut commands);

    // Добавляем -с перед каждой командой
    let args = commands
        .iter()
        .flat_map(|s| ["-c", s]);

    // Вызываем dosbox
    let child = Command::new("dosbox")
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn();

    match child {
        Ok(mut child) => child.wait(),
        Err(e) => Err(e)
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut opts = Options::new();

    // Устанавливаем параметры и флаги
    opts.optopt("c", "", 
    "Обязательный параметр. Устанавливает директорию (не путь до файла) с компилятором и компоновщиком \
    TASM. В ней должны содержаться файлы \"TASM.exe\" и \"TLINK.exe\"",
    "ПУТЬ");
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
        return;
    }

    // Флаг выхода
    let exit = matches.opt_present("e");

    // Получаем абсолютный путь до директории с компилятором
    let compiler_dir = match get_compiler_dir(&matches) {
        Ok(str) => str,
        Err(e) => {
            eprintln!("{e}");
            process::exit(-1);
        }
    };

    // Получаем абсолютный путь до компилиуемого файла
    let file_path_input = matches.free[0].clone();
    let file_path = match to_absolute_path(&file_path_input) {
        Ok(str) => str,
        Err(e) => {
            eprintln!("{e}");
            process::exit(-1);
        }
    };

    // Запуск работы с полученными параметрами
    match do_work(file_path, compiler_dir, exit) {
        Ok(status) => {
            match status.code() {
                Some(code) => process::exit(code),
                None => process::exit(-1)
            }
        },
        Err(e) => {
            println!("{e}");
            process::exit(-1);
        }
    }
}
