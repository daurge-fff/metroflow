//! Интеграционные тесты ядра: связка сети, симуляции и сохранений.

use metroflow::network::{demo_network, empty_network, StationKind};
use metroflow::save::{load_from_path, save_to_path, SaveError, SAVE_VERSION};
use metroflow::simulation::Simulation;
use std::path::PathBuf;

fn temp_path(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("metroflow_it_{}_{}.json", name, std::process::id()));
    p
}

#[test]
fn simulation_runs_on_demo_network() {
    let net = demo_network();
    assert!(net.validate().is_ok());

    let mut sim = Simulation::new_with_seed(&net, 2024);
    for _ in 0..1800 {
        // ~30 секунд при 60 FPS.
        sim.update(&net, 1.0 / 60.0);
    }

    assert!(sim.validate_against(&net).is_ok());
    assert!(sim.total_waiting() > 0, "должны появиться пассажиры");
    assert!(
        sim.total_onboard() > 0,
        "поезда должны перевозить пассажиров"
    );
    for train in &sim.trains {
        assert!(train.onboard.len() as u32 <= train.capacity);
    }
}

#[test]
fn save_load_roundtrip_preserves_state() {
    let net = demo_network();
    let mut sim = Simulation::new_with_seed(&net, 7);
    for _ in 0..600 {
        sim.update(&net, 1.0 / 60.0);
    }

    let path = temp_path("roundtrip");
    save_to_path(&path, &net, &sim).expect("сохранение должно удаться");

    let loaded = load_from_path(&path).expect("загрузка должна удаться");
    assert_eq!(loaded.network.stations.len(), net.stations.len());
    assert_eq!(loaded.network.lines.len(), net.lines.len());
    assert_eq!(loaded.simulation.trains.len(), sim.trains.len());

    let _ = std::fs::remove_file(path);
}

#[test]
fn editing_network_keeps_simulation_consistent() {
    let mut net = empty_network();
    let a = net.add_station(
        "A",
        metroflow::geom::Vec2::new(0.0, 0.0),
        StationKind::Regular,
    );
    let b = net.add_station(
        "B",
        metroflow::geom::Vec2::new(300.0, 0.0),
        StationKind::Regular,
    );
    let c = net.add_station(
        "C",
        metroflow::geom::Vec2::new(600.0, 0.0),
        StationKind::Regular,
    );
    let line = net.add_line("Л1", 0);
    net.append_station(line, a).unwrap();
    net.append_station(line, b).unwrap();

    let mut sim = Simulation::new(&net);
    sim.reconcile(&net);
    assert!(sim.trains.len() >= 2, "на новую линию добавляются поезда");
    assert!(sim.validate_against(&net).is_ok());

    // Продлеваем линию и пересогласовываем.
    net.append_station(line, c).unwrap();
    sim.reconcile(&net);
    assert!(sim.validate_against(&net).is_ok());

    // Удаляем станцию — поезда не должны сломаться.
    net.remove_station(b);
    sim.reconcile(&net);
    assert!(sim.validate_against(&net).is_ok());

    // Удаляем линию — поезда этой линии исчезают.
    net.remove_line(line);
    sim.reconcile(&net);
    assert!(sim.trains.is_empty());
    assert!(sim.validate_against(&net).is_ok());
}

#[test]
fn generated_maps_run_stably() {
    for seed in 1..12u64 {
        let params = metroflow::network::GenerateParams::random(seed);
        let net = metroflow::network::generate(params, metroflow::config::i18n::Lang::En);
        let mut sim = Simulation::new_with_seed(&net, seed);
        for _ in 0..1500 {
            sim.update(&net, 1.0 / 60.0);
        }
        assert!(sim.validate_against(&net).is_ok(), "seed {seed}");
    }
}

#[test]
fn loading_foreign_version_is_rejected() {
    let net = demo_network();
    let sim = Simulation::new_with_seed(&net, 1);
    let path = temp_path("version");
    save_to_path(&path, &net, &sim).unwrap();

    // Ставим заведомо неподдерживаемую версию.
    let content = std::fs::read_to_string(&path).unwrap();
    let tampered = content.replace(
        &format!("\"version\": {SAVE_VERSION}"),
        &format!("\"version\": {}", SAVE_VERSION + 1000),
    );
    std::fs::write(&path, tampered).unwrap();

    assert!(matches!(
        load_from_path(&path),
        Err(SaveError::UnsupportedVersion { .. })
    ));
    let _ = std::fs::remove_file(path);
}
