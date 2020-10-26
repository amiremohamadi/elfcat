use crate::elf::defs::*;
use crate::elf::parser::{Note, ParsedElf, ParsedPhdr, RangeType};
use std::fmt::Write;
use std::path::Path;

const INDENT: &str = "  ";

fn basename(path: &str) -> &str {
    // Wish expect() could use String. This is messy.
    match Path::new(path).file_name() {
        Some(name) => name.to_str().unwrap(),
        None => panic!("basename: failed for path \"{}\"", path),
    }
}

fn stem(path: &str) -> &str {
    match Path::new(path).file_stem() {
        Some(stem) => stem.to_str().unwrap(),
        None => panic!("stem: failed for path \"{}\"", path),
    }
}

pub fn construct_filename(filename: &String) -> String {
    stem(basename(filename)).to_string() + ".html"
}

fn indent(level: usize, line: &str) -> String {
    if line == "" {
        String::new()
    } else {
        INDENT.repeat(level) + line
    }
}

trait Indentable {
    fn indent_lines(&self, level: usize) -> String;
}

impl Indentable for str {
    fn indent_lines(&self, level: usize) -> String {
        self.lines().map(|x| indent(level, x) + "\n").collect()
    }
}

macro_rules! w {
    ($dst:expr, $indent_level:expr, $($arg:tt)*) => {
        wnonl!($dst, $indent_level, $( $arg )* );
        writeln!($dst, "").unwrap();
    }
}

macro_rules! wnonl {
    ($dst:expr, $indent_level:expr, $($arg:tt)*) => {
        write!($dst, "{}", INDENT.repeat($indent_level)).unwrap();
        write!($dst, $( $arg )* ).unwrap();
    }
}

fn generate_head(o: &mut String, elf: &ParsedElf) {
    let stylesheet: String = include_str!("style.css").indent_lines(3);

    w!(o, 1, "<head>");
    w!(o, 2, "<meta charset='utf-8'>");
    w!(o, 2, "<title>{}</title>", basename(&elf.filename));
    w!(o, 2, "<style>");
    wnonl!(o, 0, "{}", stylesheet);
    w!(o, 2, "</style>");
    w!(o, 1, "</head>");
}

fn generate_svg_element(o: &mut String) {
    w!(o, 2, "<svg width='100%' height='100%'>");

    w!(o, 3, "<defs>");

    wnonl!(o, 4, "<marker id='arrowhead' viewBox='0 0 10 10' ");
    wnonl!(o, 0, "refX='10' refY='5' ");
    w!(o, 0, "markerWidth='10' markerHeight='10' orient='auto'>");

    w!(o, 5, "<path d='M 0 0 L 10 5 L 0 10 z' />");

    w!(o, 4, "</marker>");

    w!(o, 3, "</defs>");

    wnonl!(o, 3, "<g id='arrows' stroke='black' ");
    wnonl!(o, 0, "stroke-width='1' marker-end='url(#arrowhead)'>");
    w!(o, 0, "</g>");

    w!(o, 2, "</svg>");
}

fn generate_info_table(o: &mut String, elf: &ParsedElf) {
    w!(o, 5, "<table>");

    for (id, desc, value) in elf.information.iter() {
        w!(o, 6, "<tr id='info_{}'>", id);

        w!(o, 7, "<td>{}:</td>", desc);
        w!(o, 7, "<td>{}</td>", value);

        w!(o, 6, "</tr>");
    }

    w!(o, 5, "</table>");
}

fn generate_phdr_info_table(o: &mut String, phdr: &ParsedPhdr, idx: usize) {
    let items = [
        ("Type", &ptype_to_string(phdr.ptype)),
        ("Flags", &phdr.flags),
        ("Offset in file", &format!("{}", phdr.file_offset)),
        ("Size in file", &format!("{}", phdr.file_size)),
        ("Vaddr in memory", &format!("{:#x}", phdr.vaddr)),
        ("Size in memory", &format!("{:#x}", phdr.memsz)),
        ("Alignment", &format!("{:#x}", phdr.alignment)),
    ];

    w!(o, 4, "<table class='conceal' id='info_phdr{}'>", idx);

    for (desc, value) in items.iter() {
        w!(o, 5, "<tr>");

        w!(o, 6, "<td>{}:</td>", desc);
        w!(o, 6, "<td>{}</td>", value);

        w!(o, 5, "</tr>");
    }

    w!(o, 4, "</table>");
}

fn generate_phdr_info_tables(o: &mut String, elf: &ParsedElf) {
    for (idx, phdr) in elf.phdrs.iter().enumerate() {
        generate_phdr_info_table(o, &phdr, idx);
    }
}

fn format_string_byte(byte: u8) -> String {
    if byte.is_ascii_graphic() {
        format!("{}", char::from(byte))
    } else {
        format!("<b>{:02x}</b> ", byte)
    }
}

fn format_string_slice(slice: &[u8]) -> String {
    slice
        .iter()
        .fold(String::new(), |s, b| s + &format_string_byte(*b))
}

fn generate_note_data(o: &mut String, note: &Note) {
    w!(o, 5, "<tr>");
    w!(o, 6, "<td>Name:</td>");
    w!(
        o,
        6,
        "<td>{}</td>",
        format_string_slice(&note.name[0..note.name.len() - 1])
    );
    w!(o, 5, "</tr>");

    w!(o, 5, "<tr>");
    w!(o, 6, "<td>Type:</td>");
    w!(o, 6, "<td>{:#x}</td>", note.ntype);
    w!(o, 5, "</tr>");

    match note.ntype {
        NT_GNU_BUILD_ID => {
            let mut hash = String::new();

            for byte in note.desc.iter() {
                append_hex_byte(&mut hash, *byte);
            }

            w!(o, 5, "<tr>");
            w!(o, 6, "<td>Build ID:</td>");
            w!(o, 6, "<td>{}</td>", hash);
            w!(o, 5, "</tr>");
        }
        _ => {
            w!(o, 5, "<tr>");
            w!(o, 6, "<td>Desc:</td>");
            w!(o, 6, "<td>{}</td>", format_string_slice(&note.desc[..]));
            w!(o, 5, "</tr>");
        }
    }
}

fn generate_segment_info_table(o: &mut String, elf: &ParsedElf, phdr: &ParsedPhdr) {
    match phdr.ptype {
        PT_INTERP => {
            w!(o, 5, "<tr>");
            w!(o, 6, "<td>Interpreter:</td>");
            w!(
                o,
                6,
                "<td>{}</td>",
                format_string_slice(
                    &elf.contents[phdr.file_offset..phdr.file_offset + phdr.file_size - 1]
                )
            );
            w!(o, 6, "</td>");
            w!(o, 5, "</tr>");
        }
        PT_NOTE => {
            for i in 0..phdr.notes.len() {
                let note = &phdr.notes[i];

                generate_note_data(o, note);

                if i != phdr.notes.len() - 1 {
                    w!(o, 5, "<tr><td><br></td></tr>");
                }
            }
        }
        _ => {}
    }
}

// this is ugly
fn has_segment_detail(ptype: u32) -> bool {
    match ptype {
        PT_INTERP => true,
        PT_NOTE => true,
        _ => false,
    }
}

fn generate_segment_info_tables(o: &mut String, elf: &ParsedElf) {
    for (idx, phdr) in elf.phdrs.iter().enumerate() {
        if !has_segment_detail(phdr.ptype) {
            continue;
        }

        w!(o, 4, "<table class='conceal' id='info_segment{}'>", idx);
        generate_segment_info_table(o, elf, &phdr);
        w!(o, 4, "</table>");
    }
}

fn generate_header(o: &mut String, elf: &ParsedElf) {
    w!(o, 2, "<table class='header'>");
    w!(o, 3, "<tr>");

    w!(o, 4, "<td>");
    generate_info_table(o, elf);
    w!(o, 4, "</td>");

    w!(o, 4, "<td id='desc'></td>");

    w!(o, 4, "<td>");
    generate_phdr_info_tables(o, elf);
    w!(o, 4, "</td>");

    w!(o, 4, "<td>");
    generate_segment_info_tables(o, elf);
    w!(o, 4, "</td>");

    w!(o, 3, "</tr>");
    w!(o, 2, "</table>");
}

fn add_highlight_script(o: &mut String) {
    let template: &str = include_str!("js/highlight.js");
    let ids = [
        "class",
        "data",
        "abi",
        "abi_ver",
        "e_type",
        "e_machine",
        "e_entry",
        "e_phoff",
        "e_shoff",
        "e_flags",
        "e_ehsize",
        "e_phentsize",
        "e_phnum",
        "e_shentsize",
        "e_shnum",
        "e_shstrndx",
    ];
    let color = "#ee9";

    w!(o, 2, "<script type='text/javascript'>");

    for id in ids.iter() {
        let info = format!("info_{}", id);

        // this is really ugly
        let code = template
            .replace("primary_id", id)
            .as_str()
            .replace("secondary_id", info.as_str())
            .replace("color", color);
        let indented: String = code.indent_lines(3);

        wnonl!(o, 0, "{}", indented);
    }

    w!(o, 2, "</script>");
}

fn add_description_script(o: &mut String) {
    w!(o, 2, "<script type='text/javascript'>");

    wnonl!(
        o,
        0,
        "{}",
        include_str!("js/description.js").indent_lines(3)
    );

    w!(o, 2, "</script>");
}

fn add_conceal_script(o: &mut String) {
    w!(o, 2, "<script type='text/javascript'>");

    wnonl!(o, 0, "{}", include_str!("js/conceal.js").indent_lines(3));

    w!(o, 2, "</script>");
}

fn add_arrows_script(o: &mut String, elf: &ParsedElf) {
    w!(o, 2, "<script type='text/javascript'>");

    wnonl!(o, 0, "{}", include_str!("js/arrows.js").indent_lines(3));

    w!(o, 3, "connect('#e_phoff', '#bin_segment0');");
    w!(o, 3, "connect('#e_shoff', '#bin_section0');");

    for i in 0..elf.phdrs.len() {
        w!(
            o,
            3,
            "connect('#bin_phdr{} > .p_offset', '#bin_segment{}');",
            i,
            i
        );
    }

    w!(o, 2, "</script>");
}

fn add_scripts(o: &mut String, elf: &ParsedElf) {
    add_highlight_script(o);

    add_description_script(o);

    add_conceal_script(o);

    add_arrows_script(o, elf);
}

fn format_magic(byte: u8) -> String {
    if byte.is_ascii_graphic() {
        format!("&nbsp;{}", char::from(byte))
    } else {
        format!("{:02x}", byte)
    }
}

fn digit_to_hex(digit: u8) -> char {
    [
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
    ][digit as usize]
}

fn append_hex_byte(s: &mut String, byte: u8) {
    if byte < 0x10 {
        s.push('0');

        s.push(digit_to_hex(byte));
    } else {
        let trailing_digit = byte % 16;
        let leading_digit = byte / 16;

        s.push(digit_to_hex(leading_digit));
        s.push(digit_to_hex(trailing_digit));
    }
}

fn generate_file_dump(elf: &ParsedElf) -> String {
    let mut dump = String::new();

    for (i, b) in elf.contents.iter().take(2000).enumerate() {
        for range_type in &elf.ranges.data[i] {
            if *range_type != RangeType::End {
                dump += format!("<span {}>", range_type.span_attributes()).as_str();
            }
        }

        if i < 4 {
            dump += format!("{}", format_magic(*b)).as_str();
        } else {
            append_hex_byte(&mut dump, *b);
        }

        for _ in 0..elf.ranges.lookup_range_ends(i) {
            dump += "</span>";
        }

        dump += if (i + 1) % 16 == 0 { "<br>\n" } else { " " };
    }

    dump
}

fn generate_body(o: &mut String, elf: &ParsedElf) {
    w!(o, 1, "<body>");

    generate_svg_element(o);

    generate_header(o, elf);

    w!(o, 2, "<div class='box'>");

    wnonl!(o, 0, "{}", generate_file_dump(elf));

    w!(o, 2, "</div>");

    add_scripts(o, elf);

    w!(o, 1, "</body>");
}

pub fn generate_report(elf: &ParsedElf) -> String {
    let mut output = String::new();

    w!(&mut output, 0, "<!doctype html>");
    w!(&mut output, 0, "<html>");

    generate_head(&mut output, elf);
    generate_body(&mut output, elf);

    w!(&mut output, 0, "</html>");

    output
}
