#[derive(Clone, Copy)]
pub enum Width {
    Fixed(usize),
    Auto,
    Expand,
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
            data,
            get_content,
        }
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
    Cells(Vec<String>),
    Headers,
}

#[derive(Default)]
pub struct Table<'a, TRow, TCol> {
    columns: Vec<Column<'a, TRow, TCol>>,
    rows: Vec<RowData>,
    title: Option<String>,
    colsep: String,
}
impl<'a, TRow, TCol> Table<'a, TRow, TCol> {
    pub fn new(columns: Vec<Column<'a, TRow, TCol>>) -> Self {
        Self {
            rows: Vec::new(),
            columns,
            title: None,
            colsep: "│".to_string(),
        }
    }

    pub fn with_colsep(mut self, colsep: &str) -> Self {
        self.colsep = colsep.to_string();
        self
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

    pub fn add_rows(&mut self, rows: impl IntoIterator<Item = TRow>) {
        self.rows.extend(rows.into_iter().map(|row| {
            RowData::Cells(
                self.columns.iter().map(|col| col.content(&row)).collect(),
            )
        }));
    }

    pub fn add_footer(&mut self, total: &TRow) {
        self.rows.push(RowData::Separator);
        self.rows.push(RowData::Cells(
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
        let mut fixed_width: usize = 0;

        for (colidx, col) in self.columns.iter_mut().enumerate() {
            match col.width {
                Width::Fixed(w) => {
                    fixed_width += w;
                    col.computed_width = w;
                }
                Width::Auto | Width::Expand => {
                    // Compute the ideal width for a column, by checking the
                    // width of each cell in this column
                    let mut w = 0;
                    for row in &self.rows {
                        match row {
                            RowData::Separator => {}
                            RowData::Headers => {
                                if let Some(t) = &col.title {
                                    w = std::cmp::max(w, t.chars().count());
                                }
                            }
                            RowData::Cells(columns) => {
                                w = std::cmp::max(w, columns[colidx].chars().count());
                            }
                        }
                    }

                    col.computed_width = w;
                    match col.width {
                        Width::Fixed(_) => {}
                        Width::Auto => {
                            fixed_width += w;
                        }
                        Width::Expand => {
                            expandable_width += w;
                            expandable_count += 1;
                        }
                    }
                }
            }
        }

        if expandable_width + fixed_width > max_width {
            if fixed_width > max_width {
                // Screen is too narrow, columns will not get what they want.
                // Assume we only give a few pixels to expandable columns then
                // apply a ratio so that the table fits
                const EXP_WIDTH: usize = 3;
                let ratio = fixed_width as f32
                    / (max_width - expandable_count * EXP_WIDTH) as f32;
                for col in self.columns.iter_mut() {
                    match col.width {
                        Width::Fixed(_) | Width::Auto => {
                            col.computed_width =
                                (col.computed_width as f32 / ratio) as usize;
                        }
                        Width::Expand => {
                            col.computed_width = EXP_WIDTH;
                        }
                    }
                }
            } else {
                let adjust =
                    (max_width - fixed_width) as f32 / expandable_width as f32;
                for col in self.columns.iter_mut() {
                    if let Width::Expand = col.width {
                        col.computed_width =
                            (col.computed_width as f32 * adjust) as usize;
                    }
                }
            }
        }
    }

    fn push_colsep(&self, into: &mut String) {
        into.push_str(&self.colsep);
    }
    fn push_rowsep(&self, into: &mut String) {
        into.push('\n');
    }

    pub fn to_string(&mut self, max_width: usize) -> String {
        let total_width =
            max_width - (self.columns.len() - 1) * self.colsep.chars().count();

        self.compute_widths(total_width);
        let mut result = String::new();

        if let Some(title) = &self.title {
            push_sep(&mut result, max_width);
            self.push_rowsep(&mut result);
            push_align(&mut result, title, max_width, Align::Center);
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
                        );
                    }
                    RowData::Cells(columns) => {
                        push_align(
                            &mut result,
                            truncate(
                                &columns[colidx],
                                col.truncate,
                                col.computed_width,
                            ),
                            col.computed_width,
                            col.align,
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
fn push_align(into: &mut String, value: &str, width: usize, align: Align) {
    match align {
        Align::Left => {
            into.push_str(&format!("{:<width$}", value, width = width))
        }
        Align::Center => {
            into.push_str(&format!("{:^width$}", value, width = width))
        }
        Align::Right => {
            into.push_str(&format!("{:>width$}", value, width = width))
        }
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
