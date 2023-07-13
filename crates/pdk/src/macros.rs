#[macro_export]
macro_rules! permutations {
	[ $( $os:path => [ $($arch:path),* ], )* ] => {
		std::collections::HashMap::from_iter([
			$(
				(
					$os,
					Vec::from_iter([
						$(
							$arch
						),*
					])
				),
			)*
		])
	};
}
