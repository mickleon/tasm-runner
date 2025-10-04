use getopts::Matches;
use std::process::{Command, ExitStatus, Stdio};
use std::{
    env,
    fs::exists,
    io,
    path::{Path, PathBuf},
};

pub struct Config {
    file_paths: Vec<PathBuf>,
    compiler_dir: PathBuf,
    copts: String,
    lopts: String,
    debug: bool,
    exit: bool,
}

impl Config {
    pub fn new(
        file_paths: Vec<PathBuf>,
        compiler_dir: PathBuf,
        copts: String,
        lopts: String,
        debug: bool,
        exit: bool,
    ) -> Config {
        Config {
            file_paths,
            compiler_dir,
            copts,
            lopts,
            debug,
            exit,
        }
    }
}

fn to_absolute_path(path_str: &str) -> Result<PathBuf, io::Error> {
    let path = Path::new(path_str);

    if path.is_absolute() {
        match path.canonicalize() {
            Ok(path) => Ok(path),
            Err(e) => return Err(io::Error::new(e.kind(), format!("{path:?}: {e}"))),
        }
    } else {
        let current_dir = env::current_dir()?;
        match current_dir.join(path).canonicalize() {
            Ok(path) => Ok(path),
            Err(e) => return Err(io::Error::new(e.kind(), format!("{path:?}: {e}"))),
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
                Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Файл {path:?} не найден"),
                ))
            }
        }
        Err(e) => {
            return Err(io::Error::new(
                e.kind(),
                format!("{path:?}: {}", e.to_string()),
            ));
        }
    }
}

// Заполняет вектор `commands` нужными командами для dosbox
fn generate_commands(config: Config, commands: &mut Vec<String>) {
    let mut file_dir = config.file_paths[0].clone();
    file_dir.pop(); // Получаем директорию файла

    let mut file_names: Vec<&str> = Vec::new();
    let mut file_stems: Vec<&str> = Vec::new();

    for file_path in config.file_paths.iter() {
        let file_name = file_path.file_name().unwrap().to_str().unwrap(); // Имя компилируемого файла

        file_names.push(file_name);

        let file_stem = file_path.file_stem().unwrap().to_str().unwrap(); // Имя компилируемого файла без расширения

        file_stems.push(file_stem);
    }

    println!("{file_names:?} {}", file_stems.join(".OBJ ").to_uppercase());

    let mut build_dir = file_dir.clone();
    build_dir.push("BUILD/"); // Получаем директорию будущего исполняемого файла

    commands.push(format!("mount C: {:?}", config.compiler_dir));
    commands.push("PATH=%PATH;C:\\".to_string());

    let working_drive = if file_dir == config.compiler_dir {
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
    commands.push(format!(
        "TASM {} ..\\{}",
        config.copts,
        file_names.join(" ..\\").to_uppercase()
    ));
    commands.push(format!(
        "TLINK {} {}.OBJ",
        config.lopts,
        file_stems.join(".OBJ ").to_uppercase()
    ));
    commands.push(format!("{}", file_stems[0].to_uppercase()));

    if config.debug {
        commands.push(format!("TD {}", file_stems[0].to_uppercase()));
    }

    if config.exit {
        commands.push("@pause".to_string());
        commands.push("exit 0".to_string());
    }
}

pub fn get_args(matches: &Matches) -> Result<Config, io::Error> {
    // Флаг выхода
    let exit = matches.opt_present("e");

    // Флаг дебага
    let debug = matches.opt_present("d");

    // Получаем параметры компилятора
    let copts = match matches.opt_str("copts") {
        Some(options) => options.to_uppercase(),
        None => "".to_string(),
    };

    // Получаем параметры компоновщика
    let mut lopts = match matches.opt_str("lopts") {
        Some(options) => options.to_uppercase(),
        None => "".to_string(),
    };

    if lopts.is_empty() {
        lopts += "/x";
    }

    // Получаем абсолютный путь до компилятора
    let compiler_dir_input = match matches.opt_str("c") {
        Some(dir) => dir,
        None => match option_env!("TASM_DIR") {
            Some(dir) => dir.to_string(),
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Путь до компилятора TASM не указан. Подробнее: --help",
                ));
            }
        },
    };

    let compiler_dir = match to_absolute_path(&compiler_dir_input) {
        Ok(compiler_absolute) => {
            let mut tasm_path = compiler_absolute.clone();
            let mut tlink_path = compiler_absolute.clone();
            tasm_path.push("TASM.exe");
            tlink_path.push("TLINK.exe");

            match check_file(tasm_path) {
                Ok(_) => {}
                Err(e) => return Err(e),
            }

            match check_file(tlink_path) {
                Ok(_) => {}
                Err(e) => return Err(e),
            }

            compiler_absolute
        }
        Err(e) => return Err(e),
    };

    // Получаем абсолютный путь до компилиуемого файла
    let mut file_paths: Vec<PathBuf> = Vec::new();
    for file_path_input in matches.free.iter() {
        let file_path = match to_absolute_path(&file_path_input) {
            Ok(path) => path,
            Err(e) => return Err(e),
        };
        file_paths.push(file_path);
    }

    Ok(Config::new(
        file_paths,
        compiler_dir,
        copts,
        lopts,
        debug,
        exit,
    ))
}

// Запуск работы
pub fn do_work(config: Config) -> Result<ExitStatus, std::io::Error> {
    let mut commands = Vec::new(); // Вектор команд для выполнения в dosbox
    generate_commands(config, &mut commands);

    // Добавляем -с перед каждой командой
    let args = commands.iter().flat_map(|s| ["-c", s]);

    // Вызываем dosbox
    let child = Command::new("dosbox")
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn();

    match child {
        Ok(mut child) => child.wait(),
        Err(e) => Err(e),
    }
}
