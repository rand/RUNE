from rune.engine import seed_facts, EvalContext, evaluate
from rune.toml_parser import parse_data_section

def test_seed_facts():
    inputs = {"a": {"b": 1}, "x": True}
    facts = seed_facts(inputs)
    assert ("val", "a.b", 1) in facts
    assert ("val", "x", True) in facts

def test_cli_smoke():
    text = open("examples/configs/basic.rune", "r", encoding="utf-8").read()
    cfg = parse_data_section(text)
    ctx = EvalContext(inputs=cfg)
    ir = evaluate(ctx).to_json_obj()
    assert "$version" in ir
