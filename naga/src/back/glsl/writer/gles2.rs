use super::*;

fn unsupported(feature: &str) -> Error {
    Error::Custom(format!("GLSL ES 1.00 does not support {feature}"))
}

impl<W: Write> Writer<'_, W> {
    pub(super) fn validate_gles2(&self) -> BackendResult {
        use crate::{Binding, BuiltIn, ScalarKind};

        if !matches!(
            self.entry_point.stage,
            ShaderStage::Vertex | ShaderStage::Fragment
        ) {
            return Err(unsupported("this shader stage"));
        }
        for (_, ty) in self.module.types.iter() {
            match ty.inner {
                TypeInner::Scalar(scalar)
                | TypeInner::Vector { scalar, .. }
                | TypeInner::Matrix { scalar, .. }
                | TypeInner::ValuePointer { scalar, .. }
                    if scalar.width != 4 && scalar.kind != ScalarKind::Bool
                        || scalar.kind == ScalarKind::Uint =>
                {
                    return Err(unsupported("unsigned integers or non-32-bit numeric types"));
                }
                TypeInner::Matrix { columns, rows, .. } if columns != rows => {
                    return Err(unsupported("non-square matrices"));
                }
                TypeInner::Atomic(_)
                | TypeInner::BindingArray { .. }
                | TypeInner::AccelerationStructure { .. }
                | TypeInner::RayQuery { .. }
                | TypeInner::CooperativeMatrix { .. } => {
                    return Err(unsupported("this resource type"));
                }
                TypeInner::Image {
                    dim,
                    arrayed,
                    class,
                } if arrayed
                    || !matches!(
                        dim,
                        crate::ImageDimension::D1
                            | crate::ImageDimension::D2
                            | crate::ImageDimension::Cube
                    )
                    || !matches!(
                        class,
                        crate::ImageClass::Sampled {
                            kind: ScalarKind::Float,
                            multi: false
                        }
                    ) =>
                {
                    return Err(unsupported(
                        "array, depth, integer, multisampled, storage or 3D textures",
                    ));
                }
                _ => {}
            }
        }
        for (_, expression) in self.module.global_expressions.iter() {
            if matches!(
                expression,
                crate::Expression::Literal(crate::Literal::U32(_))
            ) {
                return Err(unsupported("unsigned integer literals"));
            }
        }
        let check_binding = |binding: &Binding, output: bool, ty: Handle<crate::Type>| {
            match *binding {
                Binding::BuiltIn(BuiltIn::Position { .. } | BuiltIn::FrontFacing) => {}
                Binding::BuiltIn(_) => return Err(unsupported("this shader builtin")),
                Binding::Location {
                    location,
                    interpolation,
                    sampling,
                    blend_src,
                    ..
                } => {
                    if blend_src.is_some()
                        || interpolation.is_some_and(|i| i != crate::Interpolation::Perspective)
                        || sampling.is_some_and(|s| s != crate::Sampling::Center)
                    {
                        return Err(unsupported("explicit interpolation or sampling qualifiers"));
                    }
                    if self.module.types[ty].inner.scalar_kind() != Some(ScalarKind::Float) {
                        return Err(unsupported("integer shader inputs or outputs"));
                    }
                    if output && self.entry_point.stage == ShaderStage::Fragment && location != 0 {
                        return Err(unsupported("multiple fragment outputs"));
                    }
                }
            }
            Ok(())
        };
        for (ty, binding, output) in self
            .entry_point
            .function
            .arguments
            .iter()
            .map(|arg| (arg.ty, arg.binding.as_ref(), false))
            .chain(
                self.entry_point
                    .function
                    .result
                    .iter()
                    .map(|result| (result.ty, result.binding.as_ref(), true)),
            )
        {
            if let Some(binding) = binding {
                check_binding(binding, output, ty)?;
            } else if let TypeInner::Struct { ref members, .. } = self.module.types[ty].inner {
                for member in members {
                    check_binding(member.binding.as_ref().unwrap(), output, member.ty)?;
                }
            }
        }
        for (handle, global) in self.module.global_variables.iter() {
            if self.info.get_entry_point(self.entry_point_idx as usize)[handle].is_empty() {
                continue;
            }
            if !matches!(
                global.space,
                crate::AddressSpace::Private
                    | crate::AddressSpace::Uniform
                    | crate::AddressSpace::Handle
                    | crate::AddressSpace::Immediate
            ) {
                return Err(unsupported("storage or workgroup variables"));
            }
        }
        for (expressions, info) in self
            .module
            .functions
            .iter()
            .map(|(handle, function)| (&function.expressions, &self.info[handle]))
            .chain(core::iter::once((
                &self.entry_point.function.expressions,
                self.info.get_entry_point(self.entry_point_idx as usize),
            )))
        {
            for (handle, expression) in expressions.iter() {
                use crate::{BinaryOperator as Bo, Expression as E, MathFunction as Mf};
                if info[handle].ty.inner_with(&self.module.types).scalar_kind()
                    == Some(ScalarKind::Uint)
                {
                    return Err(unsupported("unsigned integers"));
                }
                match *expression {
                    E::Derivative { .. } => {
                        return Err(unsupported(
                            "derivatives without GL_OES_standard_derivatives",
                        ))
                    }
                    E::ImageLoad { .. } | E::ImageQuery { .. } => {
                        return Err(unsupported("texture loads or queries"))
                    }
                    E::Unary {
                        op: crate::UnaryOperator::BitwiseNot,
                        ..
                    }
                    | E::Binary {
                        op: Bo::ShiftLeft | Bo::ShiftRight,
                        ..
                    } => return Err(unsupported("bitwise operations")),
                    E::Binary {
                        op: Bo::And | Bo::InclusiveOr | Bo::ExclusiveOr,
                        left,
                        ..
                    } if info[left].ty.inner_with(&self.module.types).scalar_kind()
                        != Some(ScalarKind::Bool) =>
                    {
                        return Err(unsupported("integer bitwise operations"))
                    }
                    E::As { convert: None, .. } => return Err(unsupported("bitcasts")),
                    E::Relational {
                        fun: crate::RelationalFunction::IsInf | crate::RelationalFunction::IsNan,
                        ..
                    } => return Err(unsupported("infinity or NaN tests")),
                    E::Math { fun, .. }
                        if !matches!(
                            fun,
                            Mf::Abs
                                | Mf::Min
                                | Mf::Max
                                | Mf::Clamp
                                | Mf::Saturate
                                | Mf::Cos
                                | Mf::Sin
                                | Mf::Tan
                                | Mf::Acos
                                | Mf::Asin
                                | Mf::Atan
                                | Mf::Atan2
                                | Mf::Radians
                                | Mf::Degrees
                                | Mf::Ceil
                                | Mf::Floor
                                | Mf::Fract
                                | Mf::Trunc
                                | Mf::Round
                                | Mf::Exp
                                | Mf::Exp2
                                | Mf::Log
                                | Mf::Log2
                                | Mf::Pow
                                | Mf::Dot
                                | Mf::Cross
                                | Mf::Distance
                                | Mf::Length
                                | Mf::Normalize
                                | Mf::FaceForward
                                | Mf::Reflect
                                | Mf::Refract
                                | Mf::Sign
                                | Mf::Fma
                                | Mf::Mix
                                | Mf::Step
                                | Mf::SmoothStep
                                | Mf::Sqrt
                                | Mf::InverseSqrt
                                | Mf::Transpose
                        ) =>
                    {
                        return Err(unsupported("this mathematical builtin"))
                    }
                    _ => {}
                }
                if matches!(
                    info[handle].ty.inner_with(&self.module.types),
                    TypeInner::Array { .. }
                ) && !matches!(
                    expression,
                    E::GlobalVariable(_) | E::LocalVariable(_) | E::FunctionArgument(_)
                ) {
                    return Err(unsupported("array values or constructors"));
                }
            }
        }
        Ok(())
    }

    pub(super) fn write_gles2_math(
        &mut self,
        fun: crate::MathFunction,
        arg: Handle<crate::Expression>,
        arg1: Option<Handle<crate::Expression>>,
        arg2: Option<Handle<crate::Expression>>,
        ctx: &back::FunctionCtx,
    ) -> Result<bool, Error> {
        use crate::MathFunction as Mf;
        let inner = ctx.resolve_type(arg, &self.module.types);
        if fun == Mf::Transpose {
            let TypeInner::Matrix { columns, rows, .. } = *inner else {
                unreachable!()
            };
            write!(self.out, "mat{}(", rows as u8)?;
            for row in 0..rows as usize {
                for column in 0..columns as usize {
                    if row != 0 || column != 0 {
                        write!(self.out, ", ")?;
                    }
                    self.write_expr(arg, ctx)?;
                    write!(self.out, "[{column}][{row}]")?;
                }
            }
            write!(self.out, ")")?;
            return Ok(true);
        }
        if matches!(fun, Mf::Trunc | Mf::Round) {
            write!(
                self.out,
                "naga_gles2_{}(",
                if fun == Mf::Trunc { "trunc" } else { "round" }
            )?;
            self.write_expr(arg, ctx)?;
            write!(self.out, ")")?;
            return Ok(true);
        }
        if inner.scalar_kind() != Some(crate::ScalarKind::Sint)
            || !matches!(fun, Mf::Abs | Mf::Sign | Mf::Min | Mf::Max | Mf::Clamp)
        {
            return Ok(false);
        }
        self.write_value_type(inner)?;
        write!(self.out, "(")?;
        let size = match *inner {
            TypeInner::Vector { size, .. } => size as usize,
            _ => 1,
        };
        for index in 0..size {
            if index != 0 {
                write!(self.out, ", ")?;
            }
            let name = match fun {
                Mf::Abs => "abs",
                Mf::Sign => "sign",
                Mf::Min => "min",
                Mf::Max => "max",
                Mf::Clamp => "clamp",
                _ => unreachable!(),
            };
            write!(self.out, "naga_gles2_{name}(")?;
            for (argument_index, argument) in
                core::iter::once(arg).chain(arg1).chain(arg2).enumerate()
            {
                if argument_index != 0 {
                    write!(self.out, ", ")?;
                }
                self.write_expr(argument, ctx)?;
                if matches!(
                    ctx.resolve_type(argument, &self.module.types),
                    TypeInner::Vector { .. }
                ) {
                    write!(self.out, ".{}", back::COMPONENTS[index])?;
                }
            }
            write!(self.out, ")")?;
        }
        write!(self.out, ")")?;
        Ok(true)
    }

    pub(super) fn write_gles2_helpers(&mut self) -> BackendResult {
        writeln!(
            self.out,
            "int naga_gles2_abs(int x) {{ return x < 0 ? -x : x; }}"
        )?;
        writeln!(
            self.out,
            "int naga_gles2_sign(int x) {{ return x < 0 ? -1 : (x > 0 ? 1 : 0); }}"
        )?;
        writeln!(
            self.out,
            "int naga_gles2_min(int x, int y) {{ return x < y ? x : y; }}"
        )?;
        writeln!(
            self.out,
            "int naga_gles2_max(int x, int y) {{ return x > y ? x : y; }}"
        )?;
        writeln!(self.out, "int naga_gles2_clamp(int x, int lo, int hi) {{ return naga_gles2_min(naga_gles2_max(x, lo), hi); }}")?;
        for ty in ["float", "vec2", "vec3", "vec4"] {
            writeln!(
                self.out,
                "{ty} naga_gles2_trunc({ty} x) {{ return sign(x) * floor(abs(x)); }}"
            )?;
            writeln!(self.out, "{ty} naga_gles2_round({ty} x) {{ return mix(floor(x + {ty}(0.5)), {ty}(2.0) * floor(x * {ty}(0.5) + {ty}(0.5)), step(abs(fract(x) - {ty}(0.5)), {ty}(0.0))); }}")?;
        }
        Ok(())
    }

    pub(super) fn write_gles2_switch(
        &mut self,
        selector: Handle<crate::Expression>,
        cases: &[crate::SwitchCase],
        ctx: &back::FunctionCtx,
        level: back::Level,
    ) -> BackendResult {
        let selector_name = self.namer.call("switch_selector");
        let active_name = self.namer.call("switch_active");
        write!(self.out, "{level}int {selector_name} = ")?;
        self.write_expr(selector, ctx)?;
        writeln!(self.out, ";")?;
        writeln!(self.out, "{level}bool {active_name} = false;")?;
        if let Some(variable) = self.continue_ctx.enter_switch(&mut self.namer) {
            writeln!(self.out, "{level}bool {variable} = false;")?;
        }
        writeln!(self.out, "{level}do {{")?;
        for case in cases {
            write!(self.out, "{}if ({active_name} || (", level.next())?;
            match case.value {
                crate::SwitchValue::I32(value) => write!(self.out, "{selector_name} == {value}")?,
                crate::SwitchValue::U32(_) => return Err(unsupported("unsigned switch selectors")),
                crate::SwitchValue::Default => {
                    write!(self.out, "true")?;
                    for other in cases {
                        if let crate::SwitchValue::I32(value) = other.value {
                            write!(self.out, " && {selector_name} != {value}")?;
                        }
                    }
                }
            }
            writeln!(self.out, ")) {{")?;
            writeln!(self.out, "{}{active_name} = true;", level.next().next())?;
            for statement in case.body.iter() {
                self.write_stmt(statement, ctx, level.next().next())?;
            }
            if !case.fall_through
                && case
                    .body
                    .last()
                    .is_none_or(|statement| !statement.is_terminator())
            {
                writeln!(self.out, "{}break;", level.next().next())?;
            }
            writeln!(self.out, "{}}}", level.next())?;
        }
        writeln!(self.out, "{level}}} while(false);")?;
        use back::continue_forward::ExitControlFlow;
        let op = match self.continue_ctx.exit_switch() {
            ExitControlFlow::None => None,
            ExitControlFlow::Continue { variable } => Some(("continue", variable)),
            ExitControlFlow::Break { variable } => Some(("break", variable)),
        };
        if let Some((op, variable)) = op {
            writeln!(self.out, "{level}if ({variable}) {{ {op}; }}")?;
        }
        Ok(())
    }
}
