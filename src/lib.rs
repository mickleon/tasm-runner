use std::process::{Command, ExitStatus, Stdio};
use std::{env, fs::exists, io, path::{Path, PathBuf}};
use getopts::Matches;

pub struct Config {
    file_path: PathBuf,
    compiler_dir: PathBuf,
    exit: bool,
}

impl Config {
    pub fn new(file_path: PathBuf, compiler_dir: PathBuf, exit: bool) -> Config {
        Config { file_path, compiler_dir, exit }
    }
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

// Заполняет вектор `commands` нужными командами для dosbox
fn generate_commands(config: Config, commands: &mut Vec<String>) {
    let mut file_dir = config.file_path.clone();
    file_dir.pop(); // Получаем директорию файла
    
    let binding = config.file_path.clone();

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

    commands.push(format!("mount C: {:?}", config.compiler_dir));

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
    commands.push(format!("C:\\TASM.EXE {working_drive}\\{file_name}"));
    commands.push(format!("C:\\TLINK.EXE {working_drive}\\BUILD\\{file_stem}.OBJ"));
    commands.push(format!("{working_drive}\\BUILD\\{file_stem}.exe"));

    if config.exit {
        commands.push("@pause".to_string());
        commands.push("exit 0".to_string());
    }
}

pub fn get_args(matches: &Matches) -> Result<Config, io::Error> {
    // Флаг выхода
    let exit = matches.opt_present("e");

    // Получаем абсолютный путь до компилятора
    let compiler_dir_input = match matches.opt_str("c") {
        Some(path) => path,
        None => return Err(
            io::Error::new(io::ErrorKind::InvalidInput,
            "Путь до компилятора TASM не указан. Подробнее: --help"
        ))
    };
    let compiler_dir = match to_absolute_path(&compiler_dir_input) {
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

            compiler_absolute
        },
        Err(e) => return Err(e)
    };

    // Получаем абсолютный путь до компилиуемого файла
    let file_path_input = matches.free[0].clone();
    let file_path = match to_absolute_path(&file_path_input) {
        Ok(path) => path,
        Err(e) => return Err(e)
    };

    Ok(Config::new(file_path, compiler_dir, exit))

}

// Запуск работы
pub fn do_work(config: Config) -> Result<ExitStatus, std::io::Error> {
    let mut commands = Vec::new(); // Вектор команд для выполнения в dosbox
    generate_commands(config, &mut commands);

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
