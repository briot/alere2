#[derive(Clone, Copy)]
pub enum Width {
    Fixed(usize),
    Auto,
    ExpandWithMin(usize),
}
#[derive(Clone, Copy)]
pub enum Align {
    Left,
    Center,
    Right,
}
#[derive(Clone, Copy)]
pub enum Truncate {
    Left,  //  Remove left-most characters
    Right, //  Remove right-most characters
}
#[derive(Clone, Copy)]
pub enum ColumnFooter {
    Show,
    Hide,
}

pub struct Column<'a, TRow, TCol> {
    align: Align,
    truncate: Truncate,
    width: Width,
    footer: ColumnFooter,
    title: Option<String>,
    data: TCol,
    get_content: &'a dyn Fn(&TRow, &TCol) -> String,
    show_indent: bool,

    min_width: usize,
    computed_width: usize,
}
impl<'a, TRow, TCol> Column<'a, TRow, TCol> {
    pub fn new(
        data: TCol,
        get_content: &'a dyn Fn(&TRow, &TCol) -> String,
    ) -> Self {
        Self {
            align: Align::Left,
            truncate: Truncate::Right,
            width: Width::Auto,
            footer: ColumnFooter::Show,
            title: None,
            computed_width: 0,
            min_width: 0,
            show_indent: false,
            data,
            get_content,
        }
    }

    // Whether this column should show the indentation
    pub fn show_indent(mut self) -> Self {
        self.show_indent = true;
        self
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    pub fn with_footer(mut self, footer: ColumnFooter) -> Self {
        self.footer = footer;
        self
    }

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

    fn content(&self, row: &TRow) -> String {
        (self.get_content)(row, &self.data)
    }
}

#[derive(Debug)]
enum RowData {
    Separator,
    Cells(usize, Vec<String>), //  first component is the indent
    Headers,
}

#[derive(Clone)]
pub struct Settings {
    pub colsep: String,
    pub indent_size: usize,
}
impl Default for Settings {
    fn default() -> Self {
        Settings {
            colsep: "│".to_string(),
            indent_size: 1,
        }
    }
}

#[derive(Default)]
pub struct Table<'a, TRow, TCol> {
    columns: Vec<Column<'a, TRow, TCol>>,
    rows: Vec<RowData>,
    title: Option<String>,
    settings: Settings,
}
impl<'a, TRow, TCol> Table<'a, TRow, TCol> {
    pub fn new(
        columns: Vec<Column<'a, TRow, TCol>>,
        settings: &Settings,
    ) -> Self {
        Self {
            rows: Vec::new(),
            columns,
            title: None,
            settings: settings.clone(),
        }
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    pub fn with_col_headers(mut self) -> Self {
        self.rows.push(RowData::Headers);
        self.rows.push(RowData::Separator);
        self
    }

    pub fn add_rows(&mut self, rows: &[TRow], indent: usize) {
        self.rows.extend(rows.iter().map(|row| {
            RowData::Cells(
                indent,
                self.columns.iter().map(|col| col.content(row)).collect(),
            )
        }));
    }

    pub fn add_row(&mut self, row: &TRow, indent: usize) {
        self.rows.push(RowData::Cells(
            indent,
            self.columns.iter().map(|col| col.content(row)).collect(),
        ));
    }

    pub fn add_footer(&mut self, total: &TRow) {
        self.rows.push(RowData::Separator);
        self.rows.push(RowData::Cells(
            0,
            self.columns
                .iter()
                .map(|col| match col.footer {
                    ColumnFooter::Hide => String::new(),
                    ColumnFooter::Show => col.content(total),
                })
                .collect(),
        ));
    }

    /// Compute the size allocated for each column.
    /// max_width should not include the space for column separators.
    fn compute_widths(&mut self, max_width: usize) {
        let mut expandable_count: usize = 0;
        let mut expandable_width: usize = 0;
        let mut fixed_width: usize = 0; // minimal requested width

        for (colidx, col) in self.columns.iter_mut().enumerate() {
            match col.width {
                Width::Fixed(w) => {
                    fixed_width += w;
                    col.computed_width = w;
                    col.min_width = w;
                }
                Width::Auto => {
                    // Compute the ideal width for a column, by checking the
                    // width of each cell in this column
                    let mut w = 0;
                    for row in &self.rows {
                        w = std::cmp::max(
                            w,
                            match row {
                                RowData::Separator => 0,
                                RowData::Headers => {
                                    if let Some(t) = &col.title {
                                        t.chars().count()
                                    } else {
                                        0
                                    }
                                }
                                RowData::Cells(indent, columns) => {
                                    indent * self.settings.indent_size
                                        + columns[colidx].chars().count()
                                }
                            },
                        );
                    }
                    col.computed_width = w;
                    col.min_width = w;
                    fixed_width += w;
                }
                Width::ExpandWithMin(col_min) => {
                    let mut w = 0;
                    let mut min = 0;
                    for row in &self.rows {
                        w = std::cmp::max(
                            w,
                            match row {
                                RowData::Separator => 0,
                                RowData::Headers => {
                                    if let Some(t) = &col.title {
                                        t.chars().count()
                                    } else {
                                        0
                                    }
                                }
                                RowData::Cells(indent, columns) => {
                                    min = std::cmp::max(
                                        min,
                                        indent * self.settings.indent_size
                                            + col_min,
                                    );
                                    indent * self.settings.indent_size
                                        + columns[colidx].chars().count()
                                }
                            },
                        );
                    }
                    col.computed_width = w;
                    col.min_width = min;
                    expandable_width += w;
                    expandable_count += 1;
                    fixed_width += min;
                }
            }
        }

        if expandable_width + fixed_width > max_width {
            if fixed_width > max_width {
                // Screen is too narrow, so all expandable columns get their
                // minimal size, and we will occupy more than one screen line
                // per row.  Too bad.
                for col in self.columns.iter_mut() {
                    match col.width {
                        Width::Fixed(_) | Width::Auto => {}
                        Width::ExpandWithMin(_) => {
                            col.computed_width = col.min_width;
                        }
                    }
                }
            } else {
                // How much extra space do we have in each screen line ?
                let extra_width = max_width - fixed_width;

                // Divide that extra space amongst all expandable columns
                let adjust =
                    (extra_width as f32 / expandable_count as f32) as usize;

                for col in self.columns.iter_mut() {
                    if let Width::ExpandWithMin(_) = col.width {
                        col.computed_width = col.min_width + adjust;
                    }
                }
            }
        }
    }

    fn push_colsep(&self, into: &mut String) {
        into.push_str(&self.settings.colsep);
    }
    fn push_rowsep(&self, into: &mut String) {
        into.push('\n');
    }

    pub fn to_string(&mut self, max_width: usize) -> String {
        let total_width = max_width
            - (self.columns.len() - 1) * self.settings.colsep.chars().count();

        self.compute_widths(total_width);
        let mut result = String::new();

        if let Some(title) = &self.title {
            push_sep(&mut result, max_width);
            self.push_rowsep(&mut result);
            push_align(&mut result, title, max_width, Align::Center, 0);
            self.push_rowsep(&mut result);
            push_sep(&mut result, max_width);
            self.push_rowsep(&mut result);
        }

        for row in &self.rows {
            for (colidx, col) in self.columns.iter().enumerate() {
                match row {
                    RowData::Separator => {
                        push_sep(&mut result, col.computed_width);
                    }
                    RowData::Headers => {
                        push_align(
                            &mut result,
                            truncate(
                                match &col.title {
                                    None => "",
                                    Some(t) => t,
                                },
                                col.truncate,
                                col.computed_width,
                            ),
                            col.computed_width,
                            Align::Center,
                            0,
                        );
                    }
                    RowData::Cells(indent, columns) => {
                        let idt = if col.show_indent {
                            *indent * self.settings.indent_size
                        } else {
                            0
                        };
                        push_align(
                            &mut result,
                            truncate(
                                &columns[colidx],
                                col.truncate,
                                col.computed_width - idt,
                            ),
                            col.computed_width - idt,
                            col.align,
                            idt,
                        );
                    }
                }

                if colidx < self.columns.len() - 1 {
                    self.push_colsep(&mut result);
                }
            }
            self.push_rowsep(&mut result);
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
fn push_sep(into: &mut String, width: usize) {
    into.push_str(&format!("{:─^width$}", "", width = width,));
}
fn push_align(
    into: &mut String,
    value: &str,
    width: usize,
    align: Align,
    indent_chars: usize,
) {
    if indent_chars > 0 {
        into.push_str(&format!("{: <indent_chars$}", ""));
    }

    match align {
        Align::Left => into.push_str(&format!("{:<width$}", value)),
        Align::Center => into.push_str(&format!("{:^width$}", value)),
        Align::Right => into.push_str(&format!("{:>width$}", value)),
    }
}

/// Truncate the string if necessary
fn truncate(val: &str, truncate: Truncate, width: usize) -> &str {
    if val.chars().count() <= width {
        val
    } else {
        match truncate {
            Truncate::Right => trunc_keep_first(val, width),
            Truncate::Left => trunc_keep_last(val, width),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::tables::{Column, Table, Truncate, Width};

    #[test]
    fn test_table() {
        let col1_image = |row: &[&str; 2], idx: &usize| row[*idx].to_string();

        let columns = vec![
            Column::new(0, &col1_image)
                .show_indent()
                .with_width(Width::ExpandWithMin(3))
                .with_truncate(Truncate::Left),
            Column::new(1, &col1_image)
                .show_indent()
                .with_width(Width::Auto)
                .with_truncate(Truncate::Left),
        ];
        let mut table =
            Table::new(columns, &crate::tables::Settings::default());

        table.add_row(&["abcdefghijklmnopqrstuvwxyz", "123"], 0);
        table.add_row(&["abcdefghijklmn", "123456789"], 0);

        // We have plenty of space to display the columns
        assert_eq!(
            table.to_string(40),
            "abcdefghijklmnopqrstuvwxyz│123      \n\
             abcdefghijklmn            │123456789\n"
        );

        // But we can adapt to shorter widths
        assert_eq!(
            table.to_string(20),
            "qrstuvwxyz│123      \n\
             efghijklmn│123456789\n"
        );

        // until the screen is just too narrow.  First column wants 3 chars,
        // plus separator, plus 9 chars for second column.  So threshold is 13.
        assert_eq!(
            table.to_string(13),
            "xyz│123      \n\
             lmn│123456789\n"
        );

        // If really too short, too bad.  The display will not look nicer since
        // rows will occupy multiple lines.
        assert_eq!(
            table.to_string(12),
            "xyz│123      \n\
             lmn│123456789\n"
        );
        assert_eq!(
            table.to_string(1),
            "xyz│123      \n\
             lmn│123456789\n"
        );
    }
}
