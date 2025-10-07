use clap::error::ErrorKind;
use clap::{Error, Parser};
use std::process::{Command, ExitStatus, Stdio};
use std::{
    env,
    fs::exists,
    io,
    path::{Path, PathBuf},
};

#[derive(Parser)]
/// Запускает компилятор и компоновщик TASM в DOSbox для создания исполняемого
/// файла из кода в файле(ах) [FILES]... В директории исходных файлов появляется
/// поддиректория "BUILD" с исполняемым файлом
pub struct Cli {
    files: Vec<String>,

    /// Устанавливает директорию (не путь до файла) с компилятором и
    /// компоновщиком TASM. В ней должны содержаться файлы "TASM.exe" и
    /// "TLINK.exe"
    #[arg(short, value_name = "COMPILER_DIR")]
    c: Option<String>,

    /// Принимает строку, в которой содержатся параметры для TASM.exe.
    /// Например: "/X /L". Подробнее: C:\TASM
    #[arg(long, value_name = "COMPLIER_OPTIONS")]
    copts: Option<String>,

    /// Принимает строку, в которой содержатся параметры для TLINK.exe.
    /// Например: "/X /T". Подробнее: C:\TLINK
    #[arg(long, value_name = "LINKER_OPTIONS", default_value = "/X")]
    lopts: Option<String>,

    /// Запустить дебаггер TD.exe после компиляции
    #[arg(short, long)]
    debug: bool,

    /// Выйти из DOSBox после выполнения
    #[arg(short, long)]
    exit: bool,
}

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

pub fn validate_args(cli: &Cli) -> Result<String, clap::Error> {
    if cli.files.len() == 0 {
        Error::raw(
            ErrorKind::MissingRequiredArgument,
            "Укажите компилируемые файл(ы)\n",
        )
        .exit();
    }

    match cli.c.clone() {
        Some(dir) => Ok(dir),
        None => match option_env!("TASM_DIR") {
            Some(dir) => Ok(dir.to_string()),
            None => Err(Error::raw(
                ErrorKind::MissingRequiredArgument,
                "Путь до компилятора TASM не указан\n",
            )),
        },
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

    let mut build_dir = file_dir.clone();
    build_dir.push("BUILD/"); // Получаем директорию будущего исполняемого файла

    commands.push("keyb ru".to_string());
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

pub fn get_config(cli: Cli, compiler_dir_input: String) -> Result<Config, io::Error> {
    // Получаем параметры компилятора
    let copts = match cli.copts {
        Some(options) => options.to_uppercase(),
        None => "".to_string(),
    };

    // Получаем параметры компоновщика
    let lopts = match cli.lopts {
        Some(options) => options.to_uppercase(),
        None => "".to_string(),
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

    // Получаем абсолютные пути до компилиуемых файлов
    let mut file_paths: Vec<PathBuf> = Vec::new();

    for file_path_input in cli.files.iter() {
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
        cli.debug,
        cli.exit,
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
