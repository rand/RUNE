from __future__ import annotations
import json
import pathlib
import typer
from rich import print as rprint
from .toml_parser import parse_data_section
from .engine import EvalContext, evaluate
from .emitters import openai as emit_openai, anthropic as emit_anthropic, gemini as emit_gemini

app = typer.Typer(add_completion=False)

@app.command()
def check(path: str):
    text = pathlib.Path(path).read_text(encoding="utf-8")
    _ = parse_data_section(text)
    rprint("[green]OK[/green]")

@app.command()
def derive(path: str, provider: str = typer.Option("openai", "-p", "--provider")):
    text = pathlib.Path(path).read_text(encoding="utf-8")
    config = parse_data_section(text)
    ctx = EvalContext(inputs=config)
    ir = evaluate(ctx).to_json_obj()
    if provider == "openai":
        payload = emit_openai.project(ir, config)
    elif provider == "claude":
        payload = emit_anthropic.project(ir, config)
    elif provider == "gemini":
        payload = emit_gemini.project(ir, config)
    else:
        raise typer.BadParameter("Unknown provider")
    print(json.dumps(payload, indent=2))

@app.command()
def explain(path: str):
    text = pathlib.Path(path).read_text(encoding="utf-8")
    config = parse_data_section(text)
    ctx = EvalContext(inputs=config)
    ir = evaluate(ctx).to_json_obj()
    print(json.dumps(ir.get("explain", {}), indent=2))

@app.command()
def test(path: str):
    # Placeholder. Real implementation should scan [tests.*] and evaluate expectations.
    rprint("[yellow]Test runner pending implementation[/yellow]")
