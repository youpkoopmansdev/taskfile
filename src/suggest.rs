use colored::Colorize;

pub fn suggest_similar(name: &str, known_names: &[&str]) {
    let mut suggestions: Vec<(&str, usize)> = known_names
        .iter()
        .map(|k| (*k, levenshtein(name, k)))
        .filter(|(_, d)| *d <= 3)
        .collect();

    suggestions.sort_by_key(|(_, d)| *d);

    if !suggestions.is_empty() {
        let top: Vec<&str> = suggestions.iter().take(3).map(|(k, _)| *k).collect();
        eprintln!(
            "\n{} {}",
            "Did you mean:".dimmed(),
            top.iter()
                .map(|s| s.green().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let m = a.len();
    let n = b.len();

    let mut dp = vec![vec![0usize; n + 1]; m + 1];

    for (i, row) in dp.iter_mut().enumerate().take(m + 1) {
        row[0] = i;
    }
    #[allow(clippy::needless_range_loop)]
    for j in 0..=n {
        dp[0][j] = j;
    }

    for i in 1..=m {
        for j in 1..=n {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }

    dp[m][n]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levenshtein_identical() {
        assert_eq!(levenshtein("build", "build"), 0);
    }

    #[test]
    fn levenshtein_one_off() {
        assert_eq!(levenshtein("buil", "build"), 1);
        assert_eq!(levenshtein("buildd", "build"), 1);
    }

    #[test]
    fn levenshtein_different() {
        assert!(levenshtein("abc", "xyz") > 2);
    }
}
