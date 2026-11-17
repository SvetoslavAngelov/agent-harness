"""Create the Mac/Pi research report and standalone charts for publication."""
from base64 import b64encode
from html import escape
import json
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.ticker import PercentFormatter


HERE = Path(__file__).resolve().parents[1] / "docs"
CHARTS = HERE / "charts"
INK = "#16263b"
MUTED = "#465970"
COLORS = ["#257187", "#25805a", "#653896", "#d18b22"]
SETTINGS = "Mac: BF16 CPU, 30 s deadline. Pi 5: FP32 CPU, 4 threads, 60 s deadline. Luna: hosted API."
figures = {}


def read(path):
    return json.loads(path.read_text())


def axes_style(axis, percent=False):
    axis.spines[:].set_visible(False)
    axis.tick_params(axis="both", length=0, pad=8)
    axis.grid(axis="x", color="#dae4ec", linewidth=.8)
    axis.set_axisbelow(True)
    if percent:
        axis.set(xlim=(0, 1.18), xticks=[0, .25, .5, .75, 1])
        axis.xaxis.set_major_formatter(PercentFormatter(1, decimals=0))


def save_chart(figure, name, title, subtitle, footnote, **layout):
    figure.suptitle(title, x=.035, y=.97, ha="left", weight="bold", fontsize=19)
    figure.text(.035, .892, subtitle, ha="left", color=MUTED, fontsize=10.5)
    figure.text(.035, .045, footnote, va="bottom", color=MUTED, fontsize=9, linespacing=1.5)
    figure.subplots_adjust(top=.80, bottom=.23, left=.23, right=.94, wspace=.25)
    figure.subplots_adjust(**layout)
    path = CHARTS / f"{name}.png"
    figure.savefig(path, dpi=240, facecolor="white")
    plt.close(figure)
    figures[name] = "data:image/png;base64," + b64encode(path.read_bytes()).decode()


def figure_html(name, alt, caption):
    return (f'<figure><img src="{figures[name]}" alt="{escape(alt)}">'
            f'<figcaption>{caption}</figcaption></figure>')


def render():
    data = read(HERE / "report-data.json")
    models = data["systems"]
    labels = [model["label"] for model in models]
    pi = data["pi"]
    assert pi["model_server"]["runtime"]["dtype"] == "torch.float32"
    for model in models:
        assert model["attempts"] == data["benchmark"]["cases"] == 100
        assert model["totals"]["success"] == sum(cell["success"] for cell in model["by_task_type"].values())
    task_times = [model["task_seconds"] for model in models]
    CHARTS.mkdir(exist_ok=True)
    server = pi["model_server"]
    memory = [pi["harness_peak_rss_kib"]["max"] / 1024,
              server["engine_before_load"]["rss_kib"] / 1024,
              server["model_tensor_storage_bytes"] / 1024**2,
              server["warmed_idle"]["rss_kib"] / 1024,
              server["end_of_run"]["peak_rss_kib"] / 1024]

    plt.rcParams.update({"font.family": "DejaVu Sans", "font.size": 11, "text.color": INK,
                         "xtick.color": MUTED, "ytick.color": INK, "axes.labelcolor": MUTED})

    fig, axes = plt.subplots(1, 3, figsize=(12.8, 5.8), sharey=True)
    for index, (axis, metric, title) in enumerate(zip(
        axes, ("success_rate", "mean_tool_calls", "mean_requests"),
        ("Tasks completed", "Tool calls / task", "Model requests / task"), strict=True,
    )):
        values = [model[metric] for model in models]
        bars = axis.barh(range(len(models)), values, color=COLORS, height=.56)
        axis.bar_label(bars, [f"{value:.0%}" if index == 0 else f"{value:.2f}" for value in values], padding=5)
        axis.set_yticks(range(len(models)), labels)
        axis.set_title(title, loc="left", fontsize=12, weight="bold", pad=14)
        if index:
            axis.set(xlim=(0, max(values) * 1.25), xticks=range(5))
        axes_style(axis, index == 0)
    axes[0].invert_yaxis()
    save_chart(fig, "model-comparison", "FunctionGemma, from baseline to the Pi",
               "100 identical assignment tasks per system. A pass requires the correct final state and an explicit commit.",
               "All attempts are included in call averages. The base model completed 0/100 tasks.\n" + SETTINGS)

    fig, axis = plt.subplots(figsize=(11.5, 6))
    task_means = [item["mean"] for item in task_times]
    bars = axis.barh(range(len(models)), task_means, color=COLORS, height=.56)
    axis.bar_label(bars, [f"{value:.2f} s" for value in task_means], padding=7)
    axis.set(yticks=range(len(models)), yticklabels=labels, xlim=(0, max(task_means) * 1.22), xlabel="Mean seconds per task")
    axis.invert_yaxis()
    axes_style(axis)
    save_chart(fig, "task-completion-time", "How long does an assignment take?",
               "Average end-to-end task time across all 100 attempts per system, including failed attempts.",
               "The base model completed 0/100 tasks. Hosted Luna time includes service and network overhead.\n" + SETTINGS)

    fig, axis = plt.subplots(figsize=(11.5, 6))
    kinds = [("everyone", "Everyone"), ("one_user", "One user"), ("one_and_others", "One user and others")]
    for index, (model, label, color) in enumerate(zip(models, labels, COLORS, strict=True)):
        cells = [model["by_task_type"][kind] for kind, _ in kinds]
        offset = [position + (index - (len(models) - 1) / 2) * .18 for position in range(3)]
        bars = axis.barh(offset, [cell["success"] / cell["attempts"] for cell in cells],
                         height=.15, color=color, label=label)
        axis.bar_label(bars, [f"{cell['success']}/{cell['attempts']}" for cell in cells], padding=4, fontsize=9)
    axis.set(yticks=range(3), yticklabels=[label for _, label in kinds])
    axis.invert_yaxis()
    axes_style(axis, True)
    axis.legend(loc="upper left", bbox_to_anchor=(0, -.10), ncol=2, frameon=False, fontsize=9)
    save_chart(fig, "assignment-types", "Where the model succeeds",
               "Completion by assignment type on the same 100 frozen cases. Labels show passed tasks / attempted tasks.",
               SETTINGS, bottom=.30)

    fig, axis = plt.subplots(figsize=(11.5, 5.8))
    ttft = [pi["ttft_first_request"], pi["ttft_all_requests"]]
    ttft_labels = ["Initial request\n100 tasks", "All requests\n299 model calls"]
    for index, (measure, label, color) in enumerate((("mean", "Mean", COLORS[-1]), ("p95", "P95", INK))):
        bars = axis.barh([position + (index - .5) * .27 for position in range(2)],
                         [item[measure] for item in ttft], height=.23, color=color, label=label)
        axis.bar_label(bars, [f"{item[measure]:.2f} s" for item in ttft], padding=6)
    axis.set(yticks=range(2), yticklabels=ttft_labels, xlim=(0, 3.55), xlabel="Seconds to the first generated token")
    axis.invert_yaxis()
    axes_style(axis)
    axis.legend(loc="lower right", frameon=False)
    save_chart(fig, "pi-first-token", "The Pi starts generating in 1.39 seconds",
               "Mean time to first token on each task's initial request, after model warm-up.",
               "Measured from server-side prompt formatting to the first generated token ID. The HTTP response is buffered.\n"
               "Raspberry Pi 5, 8 GB RAM. FunctionGemma 270M with LoRA, FP32 CPU, 4 threads. Mac TTFT unmeasured.")

    fig, axes = plt.subplots(1, 2, figsize=(12.8, 6), gridspec_kw={"width_ratios": [1.05, 1]})
    component_labels = ["Harness\npeak RSS", "Runtime before load\nRSS", "Model tensors\nstorage size"]
    process_labels = ["Server after warm-up\nRSS", "Server lifetime peak\nRSS"]
    for axis, values, memory_labels, title in zip(
        axes, (memory[:3], memory[3:]), (component_labels, process_labels),
        ("Component measurements", "Full inference process"), strict=True,
    ):
        bars = axis.barh(range(len(values)), values, color=[COLORS[0], COLORS[-1], COLORS[1]][:len(values)], height=.5)
        axis.bar_label(bars, [f"{value:,.1f}" for value in values], padding=5, fontsize=10)
        axis.set(yticks=range(len(values)), yticklabels=memory_labels, xlim=(0, max(values) * 1.32), xlabel="MiB")
        axis.invert_yaxis()
        axis.set_title(title, loc="left", fontsize=12, weight="bold", pad=14)
        axes_style(axis)
    save_chart(fig, "pi-memory", "What fits in the Pi's memory?",
               "Raspberry Pi 5 with 8 GB RAM. FunctionGemma 270M with LoRA, FP32 CPU inference.",
               "The full inference process includes the runtime, model and working memory. Measures overlap and cannot be added.\n"
               "RSS is resident process memory. Model tensor storage counts shared storage once. 1 MiB = 1,048,576 bytes.",
               left=.20, wspace=.85, right=.96)

    model_table = "".join(
        f'<tr><th scope="row">{label}</th><td>{model["totals"]["success"]}/100</td>'
        f'<td>{model["mean_tool_calls"]:.2f}</td><td>{model["mean_requests"]:.2f}</td>'
        f'<td>{times["mean"]:.2f} s</td><td>{times["median"]:.2f} s</td><td>{times["p95"]:.2f} s</td></tr>'
        for label, model, times in zip(labels, models, task_times, strict=True)
    )
    memory_table = "".join(
        f'<tr><th scope="row">{label}</th><td>{value:,.1f} MiB</td><td>{note}</td></tr>'
        for label, value, note in zip(
            ["Rust harness", "Inference runtime before model load", "Model tensors", "Inference server after warm-up", "Inference server peak"],
            memory, ["Maximum per-task peak RSS", "RSS after Python, PyTorch and Transformers imports",
                     "FP32 parameter and buffer storage", "RSS including the loaded model and retained working memory",
                     "Lifetime peak RSS, including startup and all tasks"], strict=True,
        )
    )
    charts = [
        figure_html("model-comparison", "Completed tasks are 0, 95, 100, and 97 out of 100 for base, LoRA on Mac, Luna, and LoRA on Pi.",
                    "LoRA refers to the rank-16 adapter trained on approximately 5,000 teacher demonstrations. Call averages include every attempt."),
        figure_html("assignment-types", "The Pi completed 30/30 everyone tasks, 34/34 one-user tasks, and 33/36 mixed tasks.",
                    "The Pi's three failures all committed an incorrect final state on mixed assignments."),
        figure_html("task-completion-time", "Mean task times are " + ", ".join(f"{label} {value:.2f} seconds" for label, value in zip(labels, task_means, strict=True)),
                    "Task time covers harness launch through exit. Mac and Pi times reflect different hardware, precision and deadlines. Luna uses a hosted endpoint."),
        figure_html("pi-first-token", "On the Pi, mean initial-request TTFT was 1.39 seconds and mean all-request TTFT was 2.07 seconds.",
                    "The initial request per task is a subset of the 299 requests. Later requests include longer conversation histories. P95 uses the nearest-rank definition."),
        figure_html("pi-memory", "Harness peak 7.1 MiB, runtime before load 307.5 MiB, model storage 1,022.7 MiB, warmed server 1,646.3 MiB, peak server 2,285.8 MiB.",
                    "Component measures and process measures overlap. The model's logical tensor storage and resident process memory describe different aspects of memory use."),
    ]
    links = "".join(f'<a href="charts/{name}.png" download>{label}</a>' for name, label in (
        ("model-comparison", "Model comparison"), ("assignment-types", "Assignment types"),
        ("task-completion-time", "Task time"), ("pi-first-token", "First token"), ("pi-memory", "Memory")))
    ratio = task_times[-1]["mean"] / task_times[1]["mean"]
    html = f'''<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>FunctionGemma inference, from Mac to Raspberry Pi</title>
<style>
:root {{ --ink:#16263b; --muted:#465970; --line:#d9e3eb; }}
* {{ box-sizing:border-box; }} body {{ margin:0; color:var(--ink); background:#eff5f9; font:16px/1.65 -apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif; }}
header {{ background:#172b43; color:white; padding:3rem 1.5rem; }} .wrap, main {{ max-width:1120px; margin:auto; }}
h1 {{ font-size:clamp(2rem,4vw,3rem); line-height:1.15; margin:0 0 1.25rem; letter-spacing:-.035em; }}
header p {{ color:#dfebf7; max-width:850px; margin:.6rem 0; }} main {{ padding:1.5rem 1rem 3rem; }}
section {{ background:white; border:1px solid var(--line); border-radius:16px; padding:1.6rem; margin-bottom:1.25rem; }}
h2 {{ margin:0 0 1rem; line-height:1.3; font-size:1.5rem; }} p {{ margin:.75rem 0; }}
figure {{ margin:1.2rem 0; }} img {{ width:100%; display:block; }} figcaption {{ color:var(--muted); font-size:.92rem; margin-top:.6rem; }}
.scroll {{ overflow-x:auto; }} table {{ width:100%; border-collapse:collapse; font-size:.9rem; font-variant-numeric:tabular-nums; }}
th,td {{ padding:.8rem .6rem; border-bottom:1px solid var(--line); text-align:right; }} th:first-child {{ text-align:left; }}
thead {{ color:var(--muted); }} .metrics {{ display:grid; grid-template-columns:repeat(4,1fr); gap:1rem; margin:1rem 0; }}
.metrics p {{ padding:1rem; background:#f0f6f8; border-radius:10px; margin:0; }} .metrics strong {{ display:block; font-size:1.8rem; line-height:1.3; }}
a {{ color:#257187; text-underline-offset:3px; }} nav {{ display:flex; flex-wrap:wrap; gap:.7rem 1.4rem; }}
details {{ margin:1rem 0; }} summary {{ cursor:pointer; font-weight:600; }}
@media(max-width:700px) {{ header {{ padding:2rem 1.2rem; }} section {{ padding:1rem; }} .metrics {{ grid-template-columns:repeat(2,1fr); }} table {{ min-width:670px; }} }}
@media print {{ body {{ background:white; }} header {{ background:white; color:var(--ink); padding:0; }} header p {{ color:var(--muted); }} section {{ border:0; padding:0; }} figure {{ break-inside:avoid; }} nav {{ display:none; }} }}
</style></head><body><header><div class="wrap">
<h1>FunctionGemma inference,<br>from Mac to Raspberry Pi</h1>
<p>A 270M-parameter model learned to complete assignment tasks through tools. The LoRA checkpoint completed
95/100 tasks on the Mac and 97/100 on an 8 GB Raspberry Pi 5. The base model completed 0/100, and Luna completed 100/100.</p>
<p>100 paired cases per system · 17 November 2026</p>
</div></header><main>
<section><h2>Baseline, fine-tuning and the Luna benchmark</h2>
<p>Each task asks the model to inspect a three-person roster, update the requested assignments and commit the result. A pass requires the exact intended final state and a successful explicit commit.</p>
{charts[0]}
<div class="scroll"><table aria-label="Four-system comparison"><thead><tr><th>System</th><th>Passed</th><th>Tools / task</th><th>Requests / task</th><th>Mean task</th><th>Median</th><th>P95</th></tr></thead><tbody>{model_table}</tbody></table></div>
<p>The base model often ended early. Its shorter task times and lower call counts include incomplete attempts.</p>
{charts[1]}</section>
<section><h2>Task completion time</h2>{charts[2]}
<p>The Pi averaged {task_times[-1]["mean"]:.2f} seconds per task, compared with {task_times[1]["mean"]:.2f} seconds for the same merged checkpoint on the Mac, a {ratio:.1f}× task-time ratio. Among the Pi's 97 successful attempts, the average was {pi["task_seconds_success"]["mean"]:.2f} seconds.</p>
</section>
<section><h2>First-token latency on the Pi</h2>{charts[3]}
<p>The Pi's first model request averaged {pi["ttft_first_request"]["mean"]:.2f} seconds to its first generated token. Across all {pi["ttft_all_requests"]["count"]} model requests, the average was {pi["ttft_all_requests"]["mean"]:.2f} seconds. The server returned each HTTP response after generation completed.</p>
</section>
<section><h2>Memory on the Pi</h2>{charts[4]}
<div class="scroll"><table aria-label="Pi memory measurements"><thead><tr><th>Component or process</th><th>Memory</th><th>Measurement</th></tr></thead><tbody>{memory_table}</tbody></table></div>
<p>The on-disk checkpoint was {data["model_weight_file_bytes"]/1024**2:,.1f} MiB. At the end of the run the server used zero swap, and the firmware reported zero throttling flags. The highest temperature sampled after a task was {pi["peak_task_end_temperature_c"]:.1f} °C.</p>
</section>
<section><h2>How the comparison was measured</h2>
<p>LoRA refers to a rank-16 adapter with approximately 3.8 million trainable parameters, trained using approximately 5,000 successful task demonstrations generated by GPT Luna and merged into FunctionGemma for inference.</p>
<p>All four systems used the same 100 frozen cases, seeds 1001–1100, with the same starting rosters, tool schemas and scoring. Each system ran one attempt per case. FunctionGemma used a 256-token output cap and greedy decoding. The Pi harness and inference server both ran locally, with a fresh SQLite database per task and a separate warm-up task excluded from its reported timings.</p>
<p>The Mac FunctionGemma runs used BF16 CPU inference. The Pi used FP32 CPU inference with four threads on a Raspberry Pi 5 Model B with 8 GB RAM. Both used PyTorch 2.9.1 and Transformers 4.57.6. The Pi result used the identical merged step-1884 checkpoint from the Mac's 5,000-task evaluation. The precision and hardware differences limit conclusions about the two-point completion-rate difference.</p>
<p>The Pi used a 60-second request deadline. A 12-case pilot with the original 30-second deadline recorded four timeouts whose generations completed in 37.5–39.1 seconds. All four assignments passed with the longer deadline. The final Pi run had zero request timeouts and three incorrect-state commits. The Mac runs used a 30-second deadline.</p>
<p>Luna ran through a hosted API, with a 4,096-token output cap and up to four concurrent tasks in the original cohort. Its task times include network and service overhead. FunctionGemma ran one task at a time. Hosted model memory and Mac first-token timings were unavailable in the archived measurements.</p>
<p>The 100 cases are disjoint from training and validation examples. They had already been examined when designing the expanded dataset, so these results describe a repeat benchmark. The separately reserved 600 final test cases were unevaluated in these runs. The expanded training run combined more data, different task proportions and additional training steps.</p>
<details><summary>Measurement definitions and source records</summary>
<p>Task time runs from process launch through exit, including SQLite and local HTTP. First-token time runs from server-side prompt formatting to the first generated token ID. P95 uses nearest rank. GNU time measured each harness process's maximum RSS. Linux process files measured server RSS and its lifetime high-water mark. Tensor storage counted shared parameter storage once.</p>
<p><a href="report-data.json">Report metrics and artifact hashes</a> · <a href="report.md">Markdown report</a></p>
</details></section>
<section><h2>Download the charts</h2><p>The report embeds every chart. Individual PNGs include their own measurement notes and settings.</p><nav>{links}</nav></section>
</main></body></html>'''
    (HERE / "report.html").write_text(html)
    print(f"Created {HERE / 'report.html'} and {len(figures)} standalone charts.")


if __name__ == "__main__":
    render()
