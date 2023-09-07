use cargo::CargoResult;
use cargo::core::Package;
use cargo::core::Target;
use cargo::core::TargetKind;
use cargo::core::compiler::CompileKind;
use cargo::core::compiler::CrateType;
use cargo::core::compiler::FileType;
use cargo::core::compiler::TargetInfo;
use cargo::core::resolver::CliFeatures;
use cargo::util::command_prelude::CompileMode;

use crate::config::Config;


/// Filtered requested targets of the package.
pub type PossibleTargets<'cc> = (&'cc Package, Vec<&'cc Target>);


impl<'t> Config<'t> {
	fn members_with_features(&'t self) -> CargoResult<Vec<(&'t Package, CliFeatures)>> {
		let specs = self.compile_options.spec.to_package_id_specs(&self.workspace)?;
		let cli_features = &self.compile_options.cli_features;
		let members = self.workspace.members_with_features(&specs, cli_features)?;
		Ok(members)
	}

	/// Returns a list of targets that are requested by the user.
	/// Resolved by requested spec initially, with fallback to all possible targets.
	pub fn possible_targets_ext(&'t self) -> CargoResult<Vec<PossibleTargets<'t>>> {
		let packages = self.compile_options.spec.get_packages(&self.workspace)?;
		let mut possible = Vec::new();
		if packages.is_empty() {
			possible.extend(self.possible_targets()?);
		} else {
			possible.extend(self.possible_targets_with(packages));
		}
		Ok(possible)
	}

	/// Returns a list of potential targets that are requested by the user.
	pub fn possible_targets(&'t self) -> CargoResult<impl Iterator<Item = PossibleTargets<'t>>> {
		let packages = self.members_with_features()?.into_iter().map(|(p, _)| p);
		let members = self.possible_targets_with(packages);
		Ok(members)
	}

	pub fn possible_targets_with(&'t self,
	                             members: impl IntoIterator<Item = &'t Package>)
	                             -> impl Iterator<Item = PossibleTargets<'t>> {
		members.into_iter().filter_map(move |p| {
			                   let filter = &self.compile_options.filter;
			                   let targets = p.targets()
			                                  .into_iter()
			                                  .filter(|t| filter.is_all_targets() || filter.target_run(t))
			                                  .collect::<Vec<_>>();
			                   (!targets.is_empty()).then(|| (p, targets))
		                   })
	}

	pub fn possible_compile_kinds(&'t self) -> CargoResult<Vec<CompileKind>> {
		let member_kinds =
			self.members_with_features()?.into_iter().flat_map(|(p, _)| {
				                                         p.manifest().default_kind().into_iter().chain(
				                                                                                       p.manifest()
				                                                                                        .forced_kind()
				                                                                                        .into_iter(),
				)
			                                         });
		let mut kinds: Vec<CompileKind> = Vec::new();
		kinds.extend(&self.compile_options.build_config.requested_kinds);
		kinds.extend(member_kinds);

		if kinds.contains(&CompileKind::Host) {
			let exact = CompileKind::Target(self.host_target.clone());
			if !kinds.contains(&exact) {
				kinds.push(exact);
			}
		}
		Ok(kinds)
	}

	pub fn target_info(&self, kind: CompileKind) -> CargoResult<TargetInfo> {
		TargetInfo::new(
		                self.workspace.config(),
		                &self.possible_compile_kinds()?,
		                &self.rustc,
		                kind,
		)
	}

	/// Returns all the file types generated by rustc for the given mode/target_kind.
	///
	/// The first value is a Vec of file types generated, the second value is
	/// a list of CrateTypes that are not supported by the given target.
	// TODO: doc-copy from `cargo::core::compiler::TargetInfo.rustc_outputs`
	pub fn rustc_outputs(&self,
	                     mode: CompileMode,
	                     target_kind: &TargetKind,
	                     compile_kind: &CompileKind)
	                     -> CargoResult<(Vec<FileType>, Vec<CrateType>)> {
		let info = self.target_info_for(*compile_kind)?;
		let triple = match compile_kind {
			CompileKind::Host => &self.rustc.host,
			CompileKind::Target(target) => target.short_name(),
		};
		info.rustc_outputs(mode, target_kind, triple)
	}
}
