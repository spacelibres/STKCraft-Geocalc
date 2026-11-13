"""Check the controlled CSPICE cosine intervention against both baselines."""
import json
from pathlib import Path

directory = Path(__file__).resolve().parents[1] / 'docs/reference/native-cross-platform'
windows = json.loads((directory / 'sub-tangent-windows.json').read_text(encoding='utf8'))
linux = json.loads((directory / 'sub-tangent-linux.json').read_text(encoding='utf8'))
intervention = [json.loads(line) for line in
                (directory / 'sub-tangent-linux-windows-cos.jsonl').read_text(encoding='utf8').splitlines()]
assert len(intervention) == len(windows) == len(linux) == 7
assert intervention == windows
assert [i for i, (left, right) in enumerate(zip(intervention, linux)) if left != right] == [0, 1, 2, 3, 5]
print('All seven intervention records equal Windows; five differ from the Linux baseline.')
