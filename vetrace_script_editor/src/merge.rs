pub fn three_way_merge(base: &str, local: &str, disk: &str) -> String {
    if local == base { return disk.to_owned(); }
    if disk == base || local == disk { return local.to_owned(); }

    let base_lines = base.lines().collect::<Vec<_>>();
    let local_lines = local.lines().collect::<Vec<_>>();
    let disk_lines = disk.lines().collect::<Vec<_>>();
    let maximum = base_lines.len().max(local_lines.len()).max(disk_lines.len());
    let mut merged = String::new();
    for index in 0..maximum {
        let base_line = base_lines.get(index).copied();
        let local_line = local_lines.get(index).copied();
        let disk_line = disk_lines.get(index).copied();
        match (base_line, local_line, disk_line) {
            (_, Some(local), Some(disk)) if local == disk => push_merge_line(&mut merged, local),
            (Some(base), Some(local), Some(disk)) if local == base => push_merge_line(&mut merged, disk),
            (Some(base), Some(local), Some(disk)) if disk == base => push_merge_line(&mut merged, local),
            (None, Some(local), None) => push_merge_line(&mut merged, local),
            (None, None, Some(disk)) => push_merge_line(&mut merged, disk),
            (Some(_), None, None) => {}
            (_, local, disk) => {
                merged.push_str("<<<<<<< Studio
");
                if let Some(local) = local { push_merge_line(&mut merged, local); }
                merged.push_str("=======
");
                if let Some(disk) = disk { push_merge_line(&mut merged, disk); }
                merged.push_str(">>>>>>> Disk
");
            }
        }
    }
    if !base.ends_with('\n') && !local.ends_with('\n') && !disk.ends_with('\n') {
        merged.pop();
    }
    merged
}

fn push_merge_line(output: &mut String, line: &str) {
    output.push_str(line);
    output.push('\n');
}
