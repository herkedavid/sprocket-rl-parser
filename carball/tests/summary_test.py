from copy import deepcopy
from pathlib import Path

import pytest

import carball
from carball.decompile_replays import analyze_replay_file
from carball.json_parser.game import Game


def get_modern_replay_path() -> str:
    return str(
        Path(__file__).resolve().parents[2]
        / "test-files/psn-ids/2026-01-31.12.57 DubiousDugger Private Win 2026-01-31-12-57.replay"
    )


def test_decompile_replay_header_only_skips_network_frames():
    replay_json = carball.decompile_replay_header_only(get_modern_replay_path())
    assert replay_json.get("network_frames") is None
    assert len(replay_json["properties"]["PlayerStats"]) == 4


def test_game_initialize_summary_does_not_require_network_frames():
    replay_json = carball.decompile_replay(get_modern_replay_path())
    replay_json = deepcopy(replay_json)
    replay_json.pop("network_frames")

    game = Game()
    game.initialize(loaded_json=replay_json, parse_replay=False)

    assert len(game.players) == 4
    assert len(game.goals) == 4
    assert game.frames is None
    assert len(game.teams) == 2


def test_summarize_replay_file_returns_metadata_without_frames():
    summary = carball.summarize_replay_file(get_modern_replay_path())
    proto = summary.get_protobuf_data()

    assert proto.game_metadata.id
    assert proto.game_metadata.is_invalid_analysis is True
    assert len(proto.players) == 4
    assert len(proto.teams) == 2
    with pytest.raises(NotImplementedError):
        summary.get_data_frame()


def test_analyze_replay_file_requires_network_frames(monkeypatch):
    replay_json = carball.decompile_replay(get_modern_replay_path())
    replay_json = deepcopy(replay_json)
    replay_json["network_frames"] = None

    monkeypatch.setattr(
        "carball.decompile_replays.decompile_replay",
        lambda _: deepcopy(replay_json),
    )

    with pytest.raises(ValueError, match="summarize_replay_file"):
        analyze_replay_file(get_modern_replay_path())
