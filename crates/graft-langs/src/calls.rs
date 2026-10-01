//! Call sites of one function body (masked text). Token shapes, shared by all languages:
//! `name` (plain call), `recv.name` (call on an identifier: a module alias, a class, a
//! variable), `.name` (receiver unknown or `this`/`self`), `new:Name` (construction).

use crate::lang::Lang;
use std::collections::BTreeSet;

const MAX_CALLS: usize = 200;

const KEYWORDS: &str = "if for while switch return catch function def func sizeof typeof await yield throw \n    else do case with elif lambda and or not in is synchronized assert defined super this \n    self except finally try del global nonlocal async import from as var let const void \n    delete instanceof alignof decltype static_assert __attribute__ new class extends \n    implements throws enum union typedef operator template namespace using static extern \n    inline volatile register final public private protected abstract";

const JS_BARE: &str = "require parseInt parseFloat setTimeout setInterval clearTimeout clearInterval fetch \n    String Number Boolean Array Object Symbol BigInt Error isNaN isFinite \n    encodeURIComponent decodeURIComponent encodeURI decodeURI structuredClone \n    queueMicrotask describe it test expect beforeEach afterEach beforeAll afterAll \n    useState useEffect useRef useMemo useCallback useContext Date Map Set Promise RegExp \n    Function super import jest";
const PY_BARE: &str = "print len range int str float list dict set tuple bool open isinstance issubclass \n    getattr setattr hasattr delattr type super enumerate zip map filter sorted reversed \n    sum min max abs any all iter next repr format id input round divmod hash vars dir \n    callable property staticmethod classmethod bytes bytearray frozenset object ord chr \n    hex bin oct pow slice exec eval compile globals locals memoryview complex ascii help \n    Exception ValueError TypeError KeyError RuntimeError OSError NotImplementedError \n    AttributeError IndexError StopIteration ImportError";
const GO_BARE: &str = "go select range defer chan map interface struct package type make new len cap append \n    copy delete panic recover print println close string int int64 int32 uint uint8 uint64 \n    float64 float32 byte rune bool error complex min max clear";
const C_BARE: &str = "printf fprintf sprintf snprintf vsnprintf vfprintf scanf sscanf malloc calloc realloc \n    free memcpy memset memmove memcmp strlen strcpy strncpy strcmp strncmp strcat strncat \n    strchr strrchr strstr strdup fopen fclose fread fwrite fgets fputs fputc fseek ftell \n    fflush puts putchar getchar exit abort atoi atol atof strtol strtoul strtod assert abs \n    labs qsort bsearch rand srand time isalpha isdigit isspace isalnum isupper islower \n    toupper tolower va_start va_end va_arg static_cast dynamic_cast reinterpret_cast \n    const_cast make_unique make_shared move forward min max swap sqrt pow floor ceil fabs \n    sin cos log exp unlikely likely offsetof container_of ARRAY_SIZE";
const JAVA_BARE: &str = "super this assert";

/// Receivers that are the language's own library, not project code.
const STD_RECV: &str = "console JSON Math Object Array Promise Number String Date process Reflect Symbol \n    Buffer window document globalThis Intl System Integer Long Double Boolean List Map Set \n    Arrays Collections Objects Optional Thread Files Paths Stream Collectors std str os \n    sys re json math time datetime logging typing np pd torch tf random itertools \n    functools collections pathlib subprocess threading asyncio shutil copy abc enum \n    dataclasses fmt strconv errors io ioutil bytes sort sync atomic filepath http log \n    regexp context reflect bufio unicode utf8 exec flag url binary hex base64 rand";

/// Method names so common in the standard libraries that `x.name()` proves nothing.
const STD_METHODS: &str = "push pop shift unshift slice splice map filter reduce forEach find findIndex some \n    every includes indexOf join split trim replace match test then catch finally add \n    delete has get set keys values entries toString toLowerCase toUpperCase startsWith \n    endsWith concat sort reverse fill flat flatMap call apply bind log warn error info \n    debug on once emit off send end write read close open resolve reject all stringify \n    parse assign freeze isArray from of now padStart padEnd charAt substring substr \n    toFixed valueOf hasOwnProperty next return append extend insert remove update items \n    strip lstrip rstrip lower upper format readlines readline encode decode copy clear \n    count index isdigit isalpha title capitalize setdefault discard union intersection \n    difference exists mkdir isinstance group search sub findall fromkeys to item numpy cpu \n    cuda float int tolist reshape view size shape dim mean sum max min Error String Close \n    Write Read Lock Unlock RLock RUnlock Add Done Wait Get Set Len Less Swap Print Printf \n    Println Sprintf Errorf Fatal Fatalf Join Split Contains Trim Reset length equals \n    hashCode put isEmpty stream collect compareTo getClass println print contains getName \n    getValue getKey setName setValue iterator hasNext toList orElse isPresent ifPresent \n    asList removeIf putAll addAll subList toArray intValue build empty begin front back \n    push_back pop_back emplace_back emplace data erase reserve resize at swap c_str first \n    second reset release compare forward backward x y width height top bottom left right \n    name id value type longValue doubleValue booleanValue";

pub fn is_std_method(name: &str) -> bool {
    in_list(STD_METHODS, name)
}

pub fn is_std_receiver(name: &str) -> bool {
    in_list(STD_RECV, name)
}

fn in_list(list: &str, word: &str) -> bool {
    list.split_whitespace().any(|w| w == word)
}

fn is_id(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$' || b >= 0x80
}

fn bare_builtin(lang: Lang, name: &str) -> bool {
    in_list(KEYWORDS, name)
        || match lang {
            Lang::Js => in_list(JS_BARE, name),
            Lang::Py => in_list(PY_BARE, name),
            Lang::Go => in_list(GO_BARE, name),
            Lang::Java => in_list(JAVA_BARE, name),
            Lang::C => in_list(C_BARE, name),
        }
}

fn skip_ws(b: &[u8], mut i: usize) -> usize {
    while i < b.len() && b[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

/// Skips `<...>` generics (TS/Java/C++ call sites like `foo<T>(x)`); `i` is at `<`.
fn skip_generics(b: &[u8], mut i: usize) -> Option<usize> {
    let (start, mut depth) = (i, 0);
    while i < b.len() && i - start < 120 {
        match b[i] {
            b'<' => depth += 1,
            b'>' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            c if is_id(c) || matches!(c, b',' | b' ' | b'.' | b'[' | b']' | b'?' | b'&' | b'*') => {
            }
            _ => return None,
        }
        i += 1;
    }
    None
}

/// Reads `a.b.c` (`.`, `?.`, and for C `->` / `::`) from `i`; returns the segments and
/// the index after them.
fn read_chain(text: &str, mut i: usize) -> (Vec<&str>, usize) {
    let b = text.as_bytes();
    let mut segs = Vec::new();
    loop {
        let s = i;
        while i < b.len() && is_id(b[i]) {
            i += 1;
        }
        if i == s {
            break;
        }
        segs.push(&text[s..i]);
        let mut j = skip_ws(b, i);
        if b.get(j) == Some(&b'<') {
            if let Some(k) = skip_generics(b, j) {
                if b.get(skip_ws(b, k)) == Some(&b'(') {
                    i = k;
                    break;
                }
            }
        }
        let rest = &text[j..];
        let step = if rest.starts_with("?.") || rest.starts_with("->") || rest.starts_with("::") {
            2
        } else if rest.starts_with('.') && !rest.starts_with("..") {
            1
        } else {
            break;
        };
        j = skip_ws(b, j + step);
        if j >= b.len() || !is_id(b[j]) {
            break;
        }
        i = j;
    }
    (segs, i)
}

/// Call tokens of `body`, deduplicated, in order of first appearance.
pub fn call_tokens(lang: Lang, body: &str) -> Vec<String> {
    let b = body.as_bytes();
    let mut out: Vec<String> = Vec::new();
    let mut seen = BTreeSet::new();
    let mut push = |t: String, out: &mut Vec<String>| {
        if out.len() < MAX_CALLS && seen.insert(t.clone()) {
            out.push(t);
        }
    };
    let mut prev_word = "";
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if !(c.is_ascii_alphabetic() || c == b'_' || c == b'$' || c >= 0x80)
            || (i > 0 && is_id(b[i - 1]))
        {
            i += 1;
            continue;
        }
        let mut k = i;
        while k > 0 && b[k - 1].is_ascii_whitespace() {
            k -= 1;
        }
        let after_dot = k > 0
            && (b[k - 1] == b'.' && !(k > 1 && b[k - 2] == b'.')
                || (k > 1 && b[k - 1] == b'>' && b[k - 2] == b'-'));
        let (segs, end) = read_chain(body, i);
        let after = skip_ws(b, end);
        let is_call = b.get(after) == Some(&b'(');
        let first = segs[0];
        let last = segs[segs.len() - 1];
        if first == "new" && segs.len() == 1 {
            // `new Foo(...)` / `new a.Foo(...)`
            let (cls, cend) = read_chain(body, skip_ws(b, end));
            if let (1, Some(name)) = (cls.len(), cls.last()) {
                let call = b.get(skip_ws(b, cend)) == Some(&b'(');
                if call || lang == Lang::Js {
                    push(format!("new:{name}"), &mut out);
                }
            }
            i = cend.max(end);
            prev_word = "new";
            continue;
        }
        let declaration = matches!(prev_word, "function" | "def" | "func" | "class" | "fn");
        if is_call && !declaration {
            let upper = last.chars().next().is_some_and(char::is_uppercase);
            if after_dot {
                if !is_std_method(last) {
                    push(format!("~{last}"), &mut out);
                }
            } else if segs.len() == 1 {
                if bare_builtin(lang, last) {
                } else if upper && matches!(lang, Lang::Js | Lang::Py) {
                    push(format!("new:{last}"), &mut out);
                } else if !upper || matches!(lang, Lang::Go | Lang::C) {
                    push(last.to_string(), &mut out);
                }
            } else {
                let recv = segs[segs.len() - 2];
                if is_std_receiver(first) {
                } else if matches!(recv, "this" | "self" | "cls" | "super") {
                    push(format!(".{last}"), &mut out);
                } else if !in_list(KEYWORDS, recv) {
                    push(format!("{recv}.{last}"), &mut out);
                }
            }
        }
        prev_word = first;
        i = end.max(i + 1);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(lang: Lang, body: &str) -> Vec<String> {
        call_tokens(lang, body)
    }

    #[test]
    fn js_calls_methods_construction_and_builtins() {
        let c = t(
            Lang::Js,
            "const x = render(a); utils.merge(x); this.save(); foo().bar(); new Widget(1); console.log(x); arr.map(f); if (a) {} JSON.stringify(x); Foo();",
        );
        for want in [
            "render",
            "utils.merge",
            ".save",
            "~bar",
            "new:Widget",
            "new:Foo",
        ] {
            assert!(c.contains(&want.to_string()), "{want} in {c:?}");
        }
        for gone in ["console.log", "if", "JSON.stringify", ".map", ".log"] {
            assert!(!c.contains(&gone.to_string()), "{gone} in {c:?}");
        }
    }

    #[test]
    fn python_calls_skip_builtins_and_keep_self_methods() {
        let c = t(Lang::Py, "x = helper(a)\nself.run(b)\nprint(len(x))\nmod.func(1)\nobj = Thing()\nitems.append(2)\n");
        for want in ["helper", ".run", "mod.func", "new:Thing"] {
            assert!(c.contains(&want.to_string()), "{want} in {c:?}");
        }
        for gone in ["print", "len"] {
            assert!(!c.contains(&gone.to_string()), "{gone} in {c:?}");
        }
    }

    #[test]
    fn go_java_and_c_shapes() {
        let go = t(
            Lang::Go,
            "x := compute(a)\nfmt.Println(x)\ns.Run()\nlen(x)\nutil.Do(1)\n",
        );
        assert!(go.contains(&"compute".to_string()) && go.contains(&"util.Do".to_string()));
        assert!(!go.contains(&"len".to_string()) && !go.contains(&"fmt.Println".to_string()));
        let java = t(
            Lang::Java,
            "var a = new Parser(x); a.parse(); System.out.println(1); helper(2); this.go();",
        );
        for want in ["new:Parser", "helper", ".go"] {
            assert!(java.contains(&want.to_string()), "{want} in {java:?}");
        }
        assert!(!java.contains(&"System.out.println".to_string()));
        let c = t(
            Lang::C,
            "n = parse(buf); printf(\"x\"); p->next(1); ns::run(2); malloc(4); if (x) y();",
        );
        for want in ["parse", "p.next", "ns.run", "y"] {
            assert!(c.contains(&want.to_string()), "{want} in {c:?}");
        }
        assert!(!c.contains(&"printf".to_string()) && !c.contains(&"malloc".to_string()));
    }

    #[test]
    fn declarations_inside_bodies_are_not_calls() {
        let c = t(
            Lang::Js,
            "function inner(a) { return a; } const v = inner(1);",
        );
        assert_eq!(c, ["inner"]);
    }
}
