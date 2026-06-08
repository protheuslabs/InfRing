
fn kill_pid(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    #[cfg(unix)]
    {
        Command::new("kill")
            .arg(pid.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(windows)]
    {
        Command::new("taskkill")
            .arg("/PID")
            .arg(pid.to_string())
            .arg("/F")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(not(any(unix, windows)))]
    {
        false
    }
}

fn kill_pid_force(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    #[cfg(unix)]
    {
        Command::new("kill")
            .arg("-9")
            .arg(pid.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(windows)]
    {
        Command::new("taskkill")
            .arg("/PID")
            .arg(pid.to_string())
            .arg("/F")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(not(any(unix, windows)))]
    {
        false
    }
}

fn read_pid_file(file_path: &Path) -> Option<u32> {
    let raw = fs::read_to_string(file_path).ok()?;
    raw.trim().parse::<u32>().ok()
}

fn pid_running(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    #[cfg(unix)]
    {
        return Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    }
    #[cfg(windows)]
    {
        return Command::new("tasklist")
            .arg("/FI")
            .arg(format!("PID eq {pid}"))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .map(|out| String::from_utf8_lossy(&out.stdout).contains(&pid.to_string()))
            .unwrap_or(false);
    }
    #[cfg(not(any(unix, windows)))]
    {
        false
    }
}

fn dashboard_listener_pids(_port: u16) -> Vec<u32> {
    #[cfg(unix)]
    {
        let query = format!("TCP:{_port}");
        let output = Command::new("lsof")
            .arg("-ti")
            .arg(query)
            .arg("-sTCP:LISTEN")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output();
        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout);
            let mut pids = Vec::<u32>::new();
            for line in text.lines() {
                if let Ok(pid) = line.trim().parse::<u32>() {
                    if !pids.contains(&pid) {
                        pids.push(pid);
                    }
                }
            }
            return pids;
        }
    }
    Vec::new()
}

fn normalized_running_pids(mut pids: Vec<u32>) -> Vec<u32> {
    pids.retain(|pid| *pid > 0 && pid_running(*pid));
    pids.sort_unstable();
    pids.dedup();
    pids
}

fn pid_command_line(pid: u32) -> Option<String> {
    if pid == 0 {
        return None;
    }
    #[cfg(unix)]
    {
        return Command::new("ps")
            .arg("-o")
            .arg("command=")
            .arg("-p")
            .arg(pid.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .ok()
            .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
            .filter(|value| !value.is_empty());
    }
    #[cfg(not(unix))]
    {
        None
    }
}

fn command_pids_matching(pattern: &str) -> Vec<u32> {
    #[cfg(unix)]
    {
        let output = Command::new("pgrep")
            .arg("-f")
            .arg(pattern)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output();
        if let Ok(out) = output {
            let mut pids = Vec::<u32>::new();
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                if let Ok(pid) = line.trim().parse::<u32>() {
                    pids.push(pid);
                }
            }
            return normalized_running_pids(pids);
        }
    }
    Vec::new()
}

fn dashboard_watchdog_runtime_pids(cfg: &DashboardLaunchConfig) -> Vec<u32> {
    let mut pids = Vec::<u32>::new();
    for pid in command_pids_matching("daemon-control") {
        if let Some(cmd) = pid_command_line(pid) {
            if cmd.contains("daemon-control")
                && cmd.contains("watchdog")
                && cmd.contains(format!("--dashboard-port={}", cfg.port).as_str())
            {
                pids.push(pid);
            }
        }
    }
    normalized_running_pids(pids)
}

fn dashboard_watchdog_candidate_pids(root: &Path, cfg: &DashboardLaunchConfig) -> Vec<u32> {
    let mut pids = dashboard_watchdog_runtime_pids(cfg);
    if let Some(pid) = read_pid_file(&dashboard_watchdog_pid_path(root)) {
        pids.push(pid);
    }
    normalized_running_pids(pids)
}

fn resolve_dashboard_executable(current_exe: &Path) -> PathBuf {
    let file_name = current_exe
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let legacy_launcher = file_name.contains("openclaw");
    if !(file_name.contains("infringd") || legacy_launcher) {
        return current_exe.to_path_buf();
    }
    let ext = current_exe
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let sibling_name = if ext.is_empty() {
        "infring-ops".to_string()
    } else {
        format!("infring-ops.{ext}")
    };
    let candidate = current_exe.with_file_name(sibling_name);
    if candidate.exists() {
        candidate
    } else {
        current_exe.to_path_buf()
    }
}

fn dashboard_backend_binary_hint() -> Option<String> {
    let current_exe = std::env::current_exe().ok()?;
    let resolved = resolve_dashboard_executable(&current_exe);

    let infring_name = if cfg!(windows) {
        "infring-ops.exe"
    } else {
        "infring-ops"
    };

    let mut candidates = Vec::<PathBuf>::new();
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("target").join("debug").join(infring_name));
        candidates.push(cwd.join("target").join("release").join(infring_name));
    }
    candidates.push(resolved);

    let newest = candidates
        .into_iter()
        .filter(|path| path.is_file())
        .filter_map(|path| {
            let mtime = fs::metadata(&path)
                .ok()
                .and_then(|meta| meta.modified().ok())
                .and_then(|ts| ts.duration_since(UNIX_EPOCH).ok())
                .map(|dur| dur.as_millis())
                .unwrap_or(0);
            Some((mtime, path))
        })
        .max_by_key(|(mtime, _)| *mtime)
        .map(|(_, path)| path);

    if let Some(path) = newest {
        return Some(path.to_string_lossy().to_string());
    }
    None
}
