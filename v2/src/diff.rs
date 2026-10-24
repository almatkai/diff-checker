//! Separate direct ports: npm diff 8.0.3 line paths, and own src/lib/diff.ts trace.
use crate::{input, output, Budget, LIMIT_ERROR, LINE_LIMIT};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn tokens<'a>(a: &'a [&'a str], b: &'a [&'a str]) -> (Vec<usize>, Vec<usize>) {
    let mut ids = HashMap::new();
    let mut intern = |s: &'a str| {
        let next = ids.len();
        *ids.entry(s).or_insert(next)
    };
    (
        a.iter().map(|s| intern(s)).collect(),
        b.iter().map(|s| intern(s)).collect(),
    )
}
fn lines_ok(a: usize, b: usize) -> Result<(), String> {
    if a > LINE_LIMIT || b > LINE_LIMIT {
        Err(LIMIT_ERROR.into())
    } else {
        Ok(())
    }
}
#[derive(Clone, Copy)]
struct Component {
    kind: char,
    count: usize,
    previous: Option<usize>,
}
#[derive(Clone, Copy)]
struct Path {
    old: isize,
    last: Option<usize>,
}
fn add(
    path: Path,
    kind: char,
    count: usize,
    inc: isize,
    arena: &mut Vec<Component>,
    budget: &mut Budget,
) -> Result<Path, String> {
    budget.spend(1)?;
    let last = path.last.map(|i| arena[i]);
    let (count, previous) = match last {
        Some(c) if c.kind == kind => (c.count + count, c.previous),
        _ => (count, path.last),
    };
    arena.push(Component {
        kind,
        count,
        previous,
    });
    Ok(Path {
        old: path.old + inc,
        last: Some(arena.len() - 1),
    })
}
fn common(
    path: &mut Path,
    k: isize,
    a: &[usize],
    b: &[usize],
    arena: &mut Vec<Component>,
    budget: &mut Budget,
) -> Result<isize, String> {
    let mut new = path.old - k;
    let mut count = 0;
    while path.old + 1 < a.len() as isize && new + 1 < b.len() as isize {
        budget.spend(1)?;
        if a[(path.old + 1) as usize] != b[(new + 1) as usize] {
            break;
        }
        path.old += 1;
        new += 1;
        count += 1;
    }
    if count > 0 {
        *path = add(*path, ' ', count, 0, arena, budget)?;
    }
    Ok(new)
}
/// Exact getDiffText output, including six spaces on nonempty equal lines.
pub fn text(original: &str, changed: &str) -> Result<String, String> {
    input(original)?;
    input(changed)?;
    let a = format!("{original}\n");
    let b = format!("{changed}\n");
    lines_ok(
        a.bytes().filter(|b| *b == b'\n').count(),
        b.bytes().filter(|b| *b == b'\n').count(),
    )?;
    let old: Vec<_> = a.split_inclusive('\n').collect();
    let new: Vec<_> = b.split_inclusive('\n').collect();
    lines_ok(old.len(), new.len())?;
    let (oi, ni) = tokens(&old, &new);
    let mut budget = Budget::default();
    let mut arena = Vec::new();
    let max = old.len().checked_add(new.len()).ok_or(LIMIT_ERROR)?;
    let offset = max + 1;
    let mut best = vec![
        None;
        max.checked_mul(2)
            .and_then(|n| n.checked_add(3))
            .ok_or(LIMIT_ERROR)?
    ];
    let mut path = Path {
        old: -1,
        last: None,
    };
    let new_pos = common(&mut path, 0, &oi, &ni, &mut arena, &mut budget)?;
    let mut done = path.old + 1 >= old.len() as isize && new_pos + 1 >= new.len() as isize;
    best[offset] = Some(path);
    let mut min = -(max as isize);
    let mut upper = max as isize;
    'outer: for d in 1..=max {
        if done {
            break;
        }
        let mut k = min.max(-(d as isize));
        let end = upper.min(d as isize);
        while k <= end {
            budget.spend(1)?;
            let idx = (offset as isize + k) as usize;
            let remove = best[idx - 1].take();
            let insert = best[idx + 1];
            let can_add = insert.is_some_and(|p| p.old - k >= 0 && p.old - k < new.len() as isize);
            let can_remove = remove.is_some_and(|p| p.old + 1 < old.len() as isize);
            if !can_add && !can_remove {
                best[idx] = None;
                k += 2;
                continue;
            }
            path = if !can_remove || (can_add && remove.unwrap().old < insert.unwrap().old) {
                add(insert.unwrap(), '+', 1, 0, &mut arena, &mut budget)?
            } else {
                add(remove.unwrap(), '-', 1, 1, &mut arena, &mut budget)?
            };
            let np = common(&mut path, k, &oi, &ni, &mut arena, &mut budget)?;
            if path.old + 1 >= old.len() as isize && np + 1 >= new.len() as isize {
                done = true;
                break 'outer;
            }
            best[idx] = Some(path);
            if path.old + 1 >= old.len() as isize {
                upper = upper.min(k - 1);
            }
            if np + 1 >= new.len() as isize {
                min = min.max(k + 1);
            }
            k += 2;
        }
    }
    if !done {
        return Err(LIMIT_ERROR.into());
    }
    let mut components = Vec::new();
    let mut last = path.last;
    while let Some(i) = last {
        components.push(arena[i]);
        last = arena[i].previous;
    }
    components.reverse();
    let (mut x, mut y) = (0, 0);
    let mut out = String::new();
    for c in components {
        let value = if c.kind == '-' {
            old[x..x + c.count].concat()
        } else {
            new[y..y + c.count].concat()
        };
        if c.kind != '+' {
            x += c.count;
        }
        if c.kind != '-' {
            y += c.count;
        }
        for (i, line) in value.split('\n').enumerate() {
            if i > 0 {
                out.push('\n');
            }
            if !line.is_empty() {
                out.push_str(match c.kind {
                    '+' => ">>> ",
                    '-' => "<<< ",
                    _ => "      ",
                });
                out.push_str(line);
            }
        }
    }
    // Both augmented inputs terminate with LF; JS substring removes that final code unit.
    out.pop();
    output(out)
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiffResult {
    pub markdown: String,
    pub full_diff: String,
    pub additions: usize,
    pub removals: usize,
    pub original_formatted: String,
    pub modified_formatted: String,
}
#[derive(Clone)]
struct Edit<'a> {
    kind: char,
    line: &'a str,
}
fn myers<'a>(a: &[&'a str], b: &[&'a str]) -> Result<Vec<Edit<'a>>, String> {
    lines_ok(a.len(), b.len())?;
    let (oi, ni) = tokens(a, b);
    let mut budget = Budget::default();
    let max = a.len().checked_add(b.len()).ok_or(LIMIT_ERROR)?;
    let o = max + 1;
    let mut v = vec![0isize; 2 * max + 3];
    let mut trace = Vec::new();
    for d in 0..=max {
        // Charge the entire snapshot BEFORE allocating; the bound covers trace memory.
        budget.spend(2 * d + 3)?;
        trace.push(v[o - d - 1..=o + d + 1].to_vec());
        for k in (-(d as isize)..=d as isize).step_by(2) {
            budget.spend(1)?;
            let i = (o as isize + k) as usize;
            let mut x = if k == -(d as isize) || (k != d as isize && v[i - 1] < v[i + 1]) {
                v[i + 1]
            } else {
                v[i - 1] + 1
            };
            let mut y = x - k;
            while x < a.len() as isize && y < b.len() as isize {
                budget.spend(1)?;
                if oi[x as usize] != ni[y as usize] {
                    break;
                }
                x += 1;
                y += 1;
            }
            v[i] = x;
            if x >= a.len() as isize && y >= b.len() as isize {
                let (mut x, mut y) = (a.len() as isize, b.len() as isize);
                let mut edits = Vec::new();
                for (d, v) in trace.iter().enumerate().rev() {
                    let k = x - y;
                    let get = |k: isize| -> isize {
                        let i = k + d as isize + 1;
                        if i < 0 {
                            0
                        } else {
                            v.get(i as usize).copied().unwrap_or(0)
                        }
                    };
                    let prev_k =
                        if k == -(d as isize) || (k != d as isize && get(k - 1) < get(k + 1)) {
                            k + 1
                        } else {
                            k - 1
                        };
                    let px = get(prev_k);
                    let py = px - prev_k;
                    while x > px && y > py {
                        x -= 1;
                        y -= 1;
                        edits.push(Edit {
                            kind: ' ',
                            line: a[x as usize],
                        });
                    }
                    if d > 0 {
                        if x == px {
                            y -= 1;
                            edits.push(Edit {
                                kind: '+',
                                line: b[y as usize],
                            });
                        } else {
                            x -= 1;
                            edits.push(Edit {
                                kind: '-',
                                line: a[x as usize],
                            });
                        }
                    }
                }
                edits.reverse();
                return Ok(edits);
            }
        }
    }
    Ok(Vec::new())
}
fn edit_line(e: &Edit<'_>) -> String {
    format!(
        "{}{}",
        match e.kind {
            '+' => "+ ",
            '-' => "- ",
            _ => "  ",
        },
        e.line
    )
}
pub fn json(original: &str, modified: &str) -> Result<DiffResult, String> {
    let original_formatted = crate::json::format(original)?;
    let modified_formatted = crate::json::format(modified)?;
    lines_ok(
        original_formatted.bytes().filter(|b| *b == b'\n').count() + 1,
        modified_formatted.bytes().filter(|b| *b == b'\n').count() + 1,
    )?;
    let a: Vec<_> = original_formatted.split('\n').collect();
    let b: Vec<_> = modified_formatted.split('\n').collect();
    let edits = myers(&a, &b)?;
    let additions = edits.iter().filter(|e| e.kind == '+').count();
    let removals = edits.iter().filter(|e| e.kind == '-').count();
    let unchanged = edits.len() - additions - removals;
    let (markdown, full_diff) = if additions + removals == 0 {
        let md=format!("## Comparison Result\n\nNo differences found — the JSON objects are identical.\n\n---\n\n```json\n{original_formatted}\n```");
        (md.clone(), md)
    } else {
        let full = format!(
            "```diff\n{}\n```",
            edits.iter().map(edit_line).collect::<Vec<_>>().join("\n")
        );
        let mut shown = vec![false; edits.len()];
        for (i, e) in edits.iter().enumerate() {
            if e.kind != ' ' {
                for v in &mut shown[i.saturating_sub(3)..=(i + 3).min(edits.len() - 1)] {
                    *v = true;
                }
            }
        }
        let indices: Vec<_> = shown
            .iter()
            .enumerate()
            .filter_map(|(i, b)| b.then_some(i))
            .collect();
        for pair in indices.windows(2) {
            let gap = pair[1] - pair[0] - 1;
            if gap > 0 && gap < 4 {
                shown[pair[0] + 1..pair[1]].fill(true);
            }
        }
        let first = shown.iter().position(|v| *v).unwrap_or(0);
        let last = shown.iter().rposition(|v| *v).unwrap_or(edits.len() - 1);
        let after = edits.len() - 1 - last;
        let mut md=format!("## Comparison Result\n\n**{additions}** added | **{removals}** removed | **{unchanged}** unchanged\n\n---\n\n");
        if first >= 4 {
            md.push_str(&format!("> **{first} unchanged lines above**\n\n"));
        }
        let mut i = if first < 4 { 0 } else { first };
        while i <= last {
            let start = i;
            while i <= last && (shown[i] || i < first) {
                i += 1;
            }
            let end = if i > last && after > 0 && after < 4 {
                edits.len()
            } else {
                i
            };
            md.push_str(&format!(
                "```diff\n{}\n```\n",
                edits[start..end]
                    .iter()
                    .map(edit_line)
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
            let gap = i;
            while i <= last && !shown[i] {
                i += 1;
            }
            if i > gap {
                md.push_str(&format!("\n> **{} unchanged lines hidden**\n\n", i - gap));
            }
        }
        if after >= 4 {
            md.push_str(&format!("\n> **{after} unchanged lines below**\n"));
        }
        (md, full)
    };
    Ok(DiffResult {
        markdown: output(markdown)?,
        full_diff: output(full_diff)?,
        additions,
        removals,
        original_formatted,
        modified_formatted,
    })
}
/// Render-only adaptive fences, per block so a long run cannot amplify all chunks.
/// Raw source markdown/copy output remains byte-exact. CR is visible, not parsed.
pub fn render_markdown(raw: &str) -> Result<String, String> {
    fn push(out: &mut String, part: &str) -> Result<(), String> {
        if out.len().checked_add(part.len()).ok_or(LIMIT_ERROR)? > crate::OUTPUT_LIMIT {
            return Err(LIMIT_ERROR.into());
        }
        out.push_str(part);
        Ok(())
    }
    let lines: Vec<_> = raw.split_inclusive('\n').collect();
    let mut out = String::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let opening = line.trim_end_matches('\n');
        if !matches!(opening, "```diff" | "```json" | "  ```") {
            push(&mut out, &line.replace('\r', "\\r"))?;
            i += 1;
            continue;
        }
        let closing = if opening == "  ```" { "  ```" } else { "```" };
        let start = i;
        i += 1;
        while i < lines.len() && lines[i].trim_end_matches('\n') != closing {
            i += 1;
        }
        let end = i;
        let mut longest: usize = 0;
        let mut run = 0;
        for line in &lines[start + 1..end] {
            for c in line.chars() {
                if c == '`' {
                    run += 1;
                    longest = longest.max(run);
                } else {
                    run = 0;
                }
            }
        }
        let fence = if longest > 3 {
            "`".repeat(longest.checked_add(1).ok_or(LIMIT_ERROR)?)
        } else {
            "```".into()
        };
        push(&mut out, &lines[start].replacen("```", &fence, 1))?;
        for line in &lines[start + 1..end] {
            push(&mut out, &line.replace('\r', "\\r"))?;
        }
        if i < lines.len() {
            push(&mut out, &lines[i].replacen("```", &fence, 1))?;
            i += 1;
        }
    }
    Ok(out)
}
