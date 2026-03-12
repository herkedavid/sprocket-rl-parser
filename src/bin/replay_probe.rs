use boxcars::{HeaderProp, NetworkParse, ParserBuilder, Replay};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::Path;

#[derive(Debug)]
struct ReplaySummary {
    path: String,
    header_only: Replay,
    parsed_frames: Result<usize, String>,
}

fn parse_header_only(data: &[u8]) -> Result<Replay, String> {
    ParserBuilder::new(data)
        .with_network_parse(NetworkParse::Never)
        .on_error_check_crc()
        .parse()
        .map_err(|err| err.to_string())
}

fn parse_full(path: &str, data: &[u8]) -> Result<usize, String> {
    ParserBuilder::new(data)
        .with_network_parse(NetworkParse::Always)
        .on_error_check_crc()
        .parse()
        .map(|replay| {
            replay
                .network_frames
                .map(|frames| frames.frames.len())
                .unwrap_or(0)
        })
        .map_err(|err| format!("{}: {}", path, err))
}

fn summarize(path: &str) -> Result<ReplaySummary, String> {
    let data = fs::read(path).map_err(|err| format!("{}: {}", path, err))?;
    let header_only = parse_header_only(&data)?;
    let parsed_frames = parse_full(path, &data);
    Ok(ReplaySummary {
        path: path.to_string(),
        header_only,
        parsed_frames,
    })
}

fn header_int(replay: &Replay, key: &str) -> Option<i32> {
    replay.properties.iter().find_map(|(name, value)| {
        if name == key {
            match value {
                HeaderProp::Int(value) => Some(*value),
                _ => None,
            }
        } else {
            None
        }
    })
}

fn header_string<'a>(replay: &'a Replay, key: &str) -> Option<&'a str> {
    replay.properties.iter().find_map(|(name, value)| {
        if name == key {
            match value {
                HeaderProp::Name(value) | HeaderProp::Str(value) => Some(value.as_str()),
                _ => None,
            }
        } else {
            None
        }
    })
}

fn object_name<'a>(replay: &'a Replay, object_ind: i32) -> Option<&'a str> {
    replay.objects.get(object_ind as usize).map(String::as_str)
}

fn net_cache_class_names(replay: &Replay) -> BTreeSet<String> {
    replay
        .net_cache
        .iter()
        .filter_map(|cache| object_name(replay, cache.object_ind))
        .map(str::to_owned)
        .collect()
}

fn net_cache_attribute_names(replay: &Replay) -> BTreeSet<String> {
    replay
        .net_cache
        .iter()
        .flat_map(|cache| cache.properties.iter())
        .filter_map(|prop| object_name(replay, prop.object_ind))
        .map(str::to_owned)
        .collect()
}

fn print_summary(summary: &ReplaySummary) {
    let replay = &summary.header_only;
    let file_name = Path::new(&summary.path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(summary.path.as_str());
    let hinted_frames = header_int(replay, "NumFrames");
    let map_name = header_string(replay, "MapName").unwrap_or("<unknown>");
    let build_version = header_string(replay, "BuildVersion").unwrap_or("<unknown>");

    println!("FILE {}", file_name);
    println!(
        "  version={} net_version={} build={}",
        format!("{}.{}", replay.major_version, replay.minor_version),
        replay
            .net_version
            .map(|value| value.to_string())
            .unwrap_or_else(|| String::from("<none>")),
        build_version
    );
    println!("  map={}", map_name);
    println!(
        "  levels={} packages={} objects={} names={} keyframes={} tick_marks={} net_cache={}",
        replay.levels.len(),
        replay.packages.len(),
        replay.objects.len(),
        replay.names.len(),
        replay.keyframes.len(),
        replay.tick_marks.len(),
        replay.net_cache.len()
    );
    println!(
        "  hinted_num_frames={}",
        hinted_frames
            .map(|value| value.to_string())
            .unwrap_or_else(|| String::from("<none>"))
    );
    match &summary.parsed_frames {
        Ok(parsed) => println!("  parsed_frames={}", parsed),
        Err(err) => println!("  parsed_frames=ERR {}", err),
    }
    if let (Some(hinted), Ok(parsed)) = (hinted_frames, &summary.parsed_frames) {
        println!("  frame_count_match={}", hinted as usize == *parsed);
    }
}

fn print_set_diff(label: &str, left: &BTreeSet<String>, right: &BTreeSet<String>) {
    println!("{}", label);
    let mut count = 0usize;
    for value in right.difference(left) {
        count += 1;
        println!("  + {}", value);
    }
    if count == 0 {
        println!("  (none)");
    }
}

fn compare(old: &ReplaySummary, new: &ReplaySummary) {
    println!();
    println!("COMPARE");
    println!("  old={}", old.path);
    println!("  new={}", new.path);

    let old_classes = net_cache_class_names(&old.header_only);
    let new_classes = net_cache_class_names(&new.header_only);
    let old_attributes = net_cache_attribute_names(&old.header_only);
    let new_attributes = net_cache_attribute_names(&new.header_only);

    print_set_diff("added_net_cache_classes", &old_classes, &new_classes);
    print_set_diff("removed_net_cache_classes", &new_classes, &old_classes);
    print_set_diff("added_net_attributes", &old_attributes, &new_attributes);
    print_set_diff("removed_net_attributes", &new_attributes, &old_attributes);
}

fn usage() {
    eprintln!("usage: cargo run --bin replay_probe -- <replay> [older_replay_for_compare]");
}

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.as_slice() {
        [path] => {
            let summary = summarize(path)?;
            print_summary(&summary);
            Ok(())
        }
        [old_path, new_path] => {
            let old = summarize(old_path)?;
            let new = summarize(new_path)?;
            print_summary(&old);
            println!();
            print_summary(&new);
            compare(&old, &new);
            Ok(())
        }
        _ => {
            usage();
            Err(String::from("invalid arguments"))
        }
    }
}
