//! Turn a solved discipline into on-disk artifacts: the score PMF as
//! CSV, the CDF as a text table, a JSON summary, and simple SVG charts.
//!
//! The SVG charts are emitted by hand so the crate needs no plotting or
//! font dependencies and the output renders anywhere.

use crate::policy::Solved;
use std::fs;
use std::io;
use std::path::Path;

/// Write every artifact for `solved` under `root/<key>/`.
///
/// # Errors
///
/// Returns any I/O error from creating the directory or writing a file.
///
/// # Panics
///
/// Panics if the summary fails to serialise to JSON, which cannot
/// happen for the fixed `Summary` shape.
pub fn write_outputs(solved: &Solved, root: &Path) -> io::Result<()> {
    let dir = root.join(solved.key);
    fs::create_dir_all(&dir)?;

    let pmf: Vec<(i32, f64)> =
        solved.dist.mass.iter().map(|(&s, &p)| (s, p)).collect();
    let cdf = cumulative(&pmf);

    fs::write(dir.join("pmf.csv"), pmf_csv(&pmf))?;
    fs::write(dir.join("cdf.txt"), cdf_txt(&cdf))?;
    fs::write(
        dir.join("summary.json"),
        serde_json::to_string_pretty(&solved.summary())
            .expect("summary serialises"),
    )?;
    fs::write(
        dir.join("pmf.svg"),
        bar_chart(&format!("{} - score PMF", solved.name), &pmf),
    )?;
    fs::write(
        dir.join("cdf.svg"),
        step_chart(&format!("{} - score CDF", solved.name), &cdf),
    )?;
    Ok(())
}

fn cumulative(pmf: &[(i32, f64)]) -> Vec<(i32, f64)> {
    let mut acc = 0.0;
    pmf.iter()
        .map(|&(s, p)| {
            acc += p;
            (s, acc)
        })
        .collect()
}

fn pmf_csv(pmf: &[(i32, f64)]) -> String {
    let mut out = String::from("score,probability\n");
    for &(s, p) in pmf {
        out.push_str(&format!("{s},{p:.10}\n"));
    }
    out
}

fn cdf_txt(cdf: &[(i32, f64)]) -> String {
    let mut out = String::from("score\tcdf\n");
    for &(s, c) in cdf {
        out.push_str(&format!("{s}\t{c:.10}\n"));
    }
    out
}

// --- Minimal SVG charting ---------------------------------------------

const W: f64 = 820.0;
const H: f64 = 520.0;
const LEFT: f64 = 60.0;
const RIGHT: f64 = 20.0;
const TOP: f64 = 40.0;
const BOTTOM: f64 = 50.0;

struct Axes {
    x_min: f64,
    x_max: f64,
    y_max: f64,
}

impl Axes {
    fn px(&self, x: f64) -> f64 {
        let span = (self.x_max - self.x_min).max(1.0);
        LEFT + (x - self.x_min) / span * (W - LEFT - RIGHT)
    }
    fn py(&self, y: f64) -> f64 {
        let plot_h = H - TOP - BOTTOM;
        TOP + plot_h - (y / self.y_max) * plot_h
    }
}

fn header(title: &str, axes: &Axes) -> String {
    let baseline = H - BOTTOM;
    let mut s = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{W}\" \
height=\"{H}\" font-family=\"sans-serif\">\n\
<rect width=\"{W}\" height=\"{H}\" fill=\"white\"/>\n\
<text x=\"{tx}\" y=\"24\" font-size=\"18\" text-anchor=\"middle\">\
{title}</text>\n",
        tx = W / 2.0,
    );
    // Axes.
    s.push_str(&format!(
        "<line x1=\"{l}\" y1=\"{b}\" x2=\"{r}\" y2=\"{b}\" \
stroke=\"black\"/>\n\
<line x1=\"{l}\" y1=\"{t}\" x2=\"{l}\" y2=\"{b}\" stroke=\"black\"/>\n",
        l = LEFT,
        r = W - RIGHT,
        t = TOP,
        b = baseline,
    ));
    // y axis labels at 0 and y_max.
    s.push_str(&label(LEFT - 8.0, axes.py(0.0), "0", "end"));
    s.push_str(&label(
        LEFT - 8.0,
        axes.py(axes.y_max),
        &format!("{:.3}", axes.y_max),
        "end",
    ));
    // x axis labels at min and max.
    s.push_str(&label(
        axes.px(axes.x_min),
        baseline + 18.0,
        &format!("{}", axes.x_min as i32),
        "middle",
    ));
    s.push_str(&label(
        axes.px(axes.x_max),
        baseline + 18.0,
        &format!("{}", axes.x_max as i32),
        "middle",
    ));
    s
}

fn label(x: f64, y: f64, text: &str, anchor: &str) -> String {
    format!(
        "<text x=\"{x:.1}\" y=\"{y:.1}\" font-size=\"12\" \
text-anchor=\"{anchor}\">{text}</text>\n"
    )
}

fn axes_for(points: &[(i32, f64)]) -> Axes {
    let x_min = points.first().map_or(0, |&(s, _)| s) as f64;
    let x_max = points.last().map_or(1, |&(s, _)| s) as f64;
    let y_max = points
        .iter()
        .map(|&(_, p)| p)
        .fold(0.0_f64, f64::max)
        .max(1e-9);
    Axes {
        x_min,
        x_max,
        y_max,
    }
}

fn bar_chart(title: &str, pmf: &[(i32, f64)]) -> String {
    let axes = axes_for(pmf);
    let mut s = header(title, &axes);
    let span = (axes.x_max - axes.x_min).max(1.0);
    let bar_w = ((W - LEFT - RIGHT) / span * 0.8).max(1.0);
    let base = axes.py(0.0);
    for &(score, p) in pmf {
        let cx = axes.px(score as f64);
        let top = axes.py(p);
        s.push_str(&format!(
            "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{bar_w:.1}\" \
height=\"{h:.1}\" fill=\"steelblue\"/>\n",
            x = cx - bar_w / 2.0,
            y = top,
            h = (base - top).max(0.0),
        ));
    }
    s.push_str("</svg>\n");
    s
}

fn step_chart(title: &str, cdf: &[(i32, f64)]) -> String {
    let axes = axes_for(cdf);
    let mut s = header(title, &axes);
    let mut path = String::new();
    let mut prev_y = axes.py(0.0);
    for (i, &(score, c)) in cdf.iter().enumerate() {
        let x = axes.px(score as f64);
        let y = axes.py(c);
        if i == 0 {
            path.push_str(&format!("M {x:.1} {prev_y:.1} "));
        }
        path.push_str(&format!("L {x:.1} {prev_y:.1} L {x:.1} {y:.1} "));
        prev_y = y;
    }
    s.push_str(&format!(
        "<path d=\"{path}\" fill=\"none\" stroke=\"crimson\" \
stroke-width=\"2\"/>\n"
    ));
    s.push_str("</svg>\n");
    s
}
