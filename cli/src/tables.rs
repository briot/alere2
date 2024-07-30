pub enum Width {
    Fixed(usize),
    Auto,
    Expand,
}
pub enum Align {
    Left,
    Right,
}
pub enum Truncate {
    Left,  //  Remove left-most characters
    Right, //  Remove right-most characters
}

pub struct Column {
    align: Align,
    truncate: Truncate,
    width: Width,
}
impl Default for Column {
    fn default() -> Self {
        Column {
            align: Align::Left,
            truncate: Truncate::Right,
            width: Width::Auto,
        }
    }
}
impl Column {
    pub fn with_align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    pub fn with_truncate(mut self, truncate: Truncate) -> Self {
        self.truncate = truncate;
        self
    }

    pub fn with_width(mut self, width: Width) -> Self {
        self.width = width;
        self
    }
}

enum Row {
    Separator,
    Cells(Vec<String>),
}

#[derive(Default)]
pub struct Table {
    columns: Vec<Column>,
    rows: Vec<Row>,
}
impl Table {
    fn width_from_content(&self, col: usize) -> usize {
        let mut w = 0;
        for row in &self.rows {
            match row {
                Row::Separator => {}
                Row::Cells(columns) => {
                    w = std::cmp::max(w, columns[col].len());
                }
            }
        }
        w
    }

    fn compute_widths(&self, max_width: usize) -> Vec<usize> {
        let mut widths = Vec::new();
        let mut expandable_width: usize = 0;
        let mut fixed_width: usize = 0;

        for (colidx, col) in self.columns.iter().enumerate() {
            match col.width {
                Width::Fixed(w) => {
                    fixed_width += w;
                    widths.push(w);
                }
                Width::Auto => {
                    let w = self.width_from_content(colidx);
                    fixed_width += w;
                    widths.push(w);
                }
                Width::Expand => {
                    let w = self.width_from_content(colidx);
                    expandable_width += w;
                    widths.push(w);
                }
            }
        }

        if expandable_width + fixed_width > max_width {
            let adjust =
                (max_width - fixed_width) as f32 / expandable_width as f32;
            for (colidx, col) in self.columns.iter().enumerate() {
                if let Width::Expand = col.width {
                    widths[colidx] = (widths[colidx] as f32 * adjust) as usize;
                }
            }
        }

        widths
    }

    pub fn add_column(&mut self, column: Column) {
        self.columns.push(column);
    }

    pub fn add_sep(&mut self) {
        self.rows.push(Row::Separator);
    }

    pub fn add_row(&mut self, row: Vec<String>) {
        self.rows.push(Row::Cells(row));
    }

    pub fn to_string(&self, max_width: usize) -> String {
        let total_width = max_width - self.columns.len();

        let widths = self.compute_widths(total_width);
        let mut result = String::new();

        for row in &self.rows {
            match row {
                Row::Separator => {
                    for (colidx, _col) in self.columns.iter().enumerate() {
                        let max_col_width = widths[colidx];
                        result.push_str(
                            &format!("{:-^width$}", "", width=max_col_width)
                        );
                        if colidx < self.columns.len() - 1 {
                            result.push(' ');
                        }
                    }
                }
                Row::Cells(columns) => {
                    for (colidx, content) in columns.iter().enumerate() {
                        let max_col_width = widths[colidx];
                        let col = &self.columns[colidx];
                        let truncated = if content.len() <= max_col_width {
                            content
                        } else {
                            match col.truncate {
                                Truncate::Right => {
                                    trunc_keep_first(content, max_col_width)
                                }
                                Truncate::Left => {
                                    trunc_keep_last(content, max_col_width)
                                }
                            }
                        };

                        match col.align {
                            Align::Left => {
                                result.push_str(&format!(
                                    "{:<0width$}",
                                    truncated,
                                    width = widths[colidx],
                                ));
                            }
                            Align::Right => {
                                result.push_str(&format!(
                                    "{:>0width$}",
                                    truncated,
                                    width = widths[colidx],
                                ));
                            }
                        }

                        if colidx < self.columns.len() - 1 {
                            result.push(' ');
                        }
                    }
                }
            }
            result.push('\n');
        }

        result
    }
}

fn trunc_keep_last(s: &str, max_width: usize) -> &str {
    s.char_indices()
        .rev()
        .nth(max_width - 1)
        .map_or_else(|| s, |(i, _)| &s[i..])
}
fn trunc_keep_first(s: &str, max_width: usize) -> &str {
    s.char_indices()
        .nth(max_width)
        .map_or_else(|| s, |(i, _)| &s[..i])
}
