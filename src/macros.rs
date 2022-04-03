macro_rules! spawn {
    ( $command:expr ) => {
        |rwm: &mut Rwm| rwm.spawn($command, &[])
    };
    ( $command:expr, $( $arg:expr )*, $ ) => {
        |rwm: &mut Rwm| rwm.spawn($command, &[$(arg,)*])
    };
}

macro_rules! kill {
    () => {
        |rwm: &mut Rwm| rwm.kill()
    };
}

macro_rules! swap {
    () => {
        |rwm: &mut Rwm| rwm.swap()
    };
}

macro_rules! main_factor {
    ( $factor:expr ) => {
        |rwm: &mut Rwm| rwm.main_factor($factor)
    };
}

macro_rules! toggle_fullscreen {
    () => {
        |rwm: &mut Rwm| rwm.toggle_fullscreen()
    };
}

macro_rules! toggle_floating {
    () => {
        |rwm: &mut Rwm| rwm.toggle_floating()
    };
}

macro_rules! view {
    ( $tag:expr ) => {
        |rwm: &mut Rwm| rwm.view($tag)
    };
}

macro_rules! tag {
    ( $tag:expr ) => {
        |rwm: &mut Rwm| rwm.tag($tag)
    };
}

macro_rules! tagmon {
    () => {
        |rwm: &mut Rwm| rwm.tagmon()
    };
}

macro_rules! quit {
    () => {
        |rwm: &mut Rwm| rwm.quit()
    };
}

macro_rules! drag {
    () => {
        |rwm: &mut Rwm| rwm.drag()
    };
}

macro_rules! resize {
    () => {
        |rwm: &mut Rwm| rwm.resize()
    };
}

macro_rules! count {
    () => (0);
    ( $x:tt $($xs:tt)* ) => (1 + count!($($xs)*));
}

macro_rules! tags {
    ( $( $tag:expr ),*$( , )? ) => {
        pub const TAGS: [&str; count!($($tag)*)] = [$($tag,)*];
    };
}

macro_rules! keys {
    ( $( $tup:expr ),*$( , )? ) => {
        pub const KEYS: [(KeyCombo, fn(&mut Rwm)); count!($($tup)*)] = [$((KeyCombo::new($tup.0, $tup.1), $tup.2)),*];
    };
}

macro_rules! buttons {
    ( $( $tup:expr ),*$( , )? ) => {
        pub const BUTTONS: [(ButtonCombo, fn(&mut Rwm)); count!($($tup)*)] = [$((ButtonCombo::new($tup.0, $tup.1), $tup.2)),*];
    };
}

macro_rules! atoms_index {
    ( $first:ident $( $atom:ident )* ) => {
        pub const $first: usize = 0;

        atoms_index!($($atom)*, 0);
    };
    ( $first:ident $( $atom:ident )*, $index:expr ) => {
        pub const $first: usize = $index + 1;

        atoms_index!($($atom)*, $index + 1);
    };
    ( , $index:expr ) => ();
}

macro_rules! atoms {
    ( $( $atom:ident ),*$(,)? ) => {
        atoms_index!($($atom)*);

        pub const ATOMS: [&str; count!($($atom)*)] = [$(stringify!($atom),)*];
    };
}
