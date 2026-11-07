//! Concrete chart bindings backed by immutable script data snapshots.
//!
//! `gpui_component::plot::Plot` is deliberately not registered: it is a Rust
//! painting trait implemented by concrete chart elements, not a constructible
//! component surface.
//!
//! A chart keys its hover state and path caches on an `ElementId`, and the one
//! it takes by default is its construction site — which is this file, shared by
//! every chart of that kind a script draws. So the wrapper carries the spec
//! node's own id, and the element id stack tells the charts under it apart.

use super::support::bool_method;

use gpui_component::chart::{AreaChart, BarChart, LineChart, PieChart, RadarChart};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDataValue, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, anyhow,
    gpui::{
        self, InteractiveElement as _, IntoElement as _, ParentElement as _, Refineable as _,
        RenderOnce, Styled as _,
    },
};
use std::sync::Arc;

#[derive(Clone)]
struct Payload(ComponentArgument);

#[derive(Clone)]
struct Row {
    label: String,
    value: f64,
}

#[derive(Clone, Copy)]
enum Op {
    Grid(bool),
    Dot,
    Natural,
    Linear,
    StepAfter,
    TickMargin(usize),
    Axis(bool),
    ValueAxis(bool),
    InnerRadius(f32),
    PadAngle(f32),
    Labels(bool),
    GridLevels(usize),
}

fn field<'a>(
    fields: &'a [(String, ComponentDataValue)],
    name: &str,
) -> Option<&'a ComponentDataValue> {
    fields
        .iter()
        .find_map(|(key, value)| (key == name).then_some(value))
}

fn rows(
    callback: &gpui_shell::ComponentDataCallback,
    window: &mut gpui::Window,
    cx: &mut gpui::App,
) -> anyhow::Result<Vec<Row>> {
    let snapshot = callback.snapshot_rows_with(&[], window, cx)?;
    (0..snapshot.len())
        .map(|index| {
            let ComponentDataValue::Object(fields) = snapshot.row(index)? else {
                anyhow::bail!("chart row {index} must be a plain object")
            };
            let Some(ComponentDataValue::String(label)) = field(fields, "label") else {
                anyhow::bail!("chart row {index} must have string field `label`")
            };
            let Some(ComponentDataValue::Number(value)) = field(fields, "value") else {
                anyhow::bail!("chart row {index} must have finite number field `value`")
            };
            Ok(Row {
                label: label.clone(),
                value: *value,
            })
        })
        .collect()
}

fn wrap(
    mut request: MaterializeRequest<'_>,
    chart: impl gpui::IntoElement + 'static,
) -> anyhow::Result<gpui::AnyElement> {
    anyhow::ensure!(request.children_len() == 0, "charts do not accept children");
    anyhow::ensure!(
        request.take_children()?.is_empty(),
        "charts do not accept ordinary children"
    );
    let mut wrapper = gpui::div()
        .id(request.element_id().clone())
        .size_full()
        .child(chart);
    wrapper.style().refine(&request.take_style());
    Ok(wrapper.into_any_element())
}

#[derive(gpui::IntoElement)]
struct ChartHost {
    kind: &'static str,
    callback: gpui_shell::ComponentDataCallback,
    ops: Vec<Op>,
}
impl RenderOnce for ChartHost {
    fn render(self, window: &mut gpui::Window, cx: &mut gpui::App) -> impl gpui::IntoElement {
        let data = match rows(&self.callback, window, cx) {
            Ok(data) => data,
            Err(error) => {
                #[cfg(test)]
                test_probe::error(error.to_string());
                return gpui::div()
                    .child(format!("Failed to build {} data: {error:#}", self.kind))
                    .into_any_element();
            }
        };
        match self.kind {
            "BarChart" => {
                let mut chart = BarChart::new(data)
                    .band(|row| row.label.clone())
                    .value(|row| row.value)
                    .label(|row| row.value.to_string());
                for op in &self.ops {
                    chart = match op {
                        Op::Grid(value) => chart.grid(*value),
                        Op::TickMargin(value) => chart.tick_margin(*value),
                        Op::Axis(value) => chart.label_axis(*value),
                        Op::ValueAxis(value) => chart.value_axis(*value),
                        _ => chart,
                    };
                }
                chart.into_any_element()
            }
            "LineChart" => {
                let mut chart = LineChart::new(data)
                    .x(|row| row.label.clone())
                    .y(|row| row.value);
                for op in &self.ops {
                    chart = match op {
                        Op::Grid(value) => chart.grid(*value),
                        Op::Dot => chart.dot(),
                        Op::Natural => chart.natural(),
                        Op::Linear => chart.linear(),
                        Op::StepAfter => chart.step_after(),
                        Op::TickMargin(value) => chart.tick_margin(*value),
                        Op::Axis(value) => chart.x_axis(*value),
                        _ => chart,
                    };
                }
                chart.into_any_element()
            }
            "AreaChart" => {
                let mut chart = AreaChart::new(data)
                    .x(|row| row.label.clone())
                    .y(|row| row.value);
                for op in &self.ops {
                    chart = match op {
                        Op::Grid(value) => chart.grid(*value),
                        Op::Natural => chart.natural(),
                        Op::Linear => chart.linear(),
                        Op::StepAfter => chart.step_after(),
                        Op::TickMargin(value) => chart.tick_margin(*value),
                        Op::Axis(value) => chart.x_axis(*value),
                        _ => chart,
                    };
                }
                chart.into_any_element()
            }
            "PieChart" => {
                let mut chart = PieChart::new(data).value(|row| row.value as f32);
                for op in &self.ops {
                    chart = match op {
                        Op::InnerRadius(value) => chart.inner_radius(*value),
                        Op::PadAngle(value) => chart.pad_angle(*value),
                        Op::Labels(true) => chart.label(|row| row.label.clone().into()),
                        _ => chart,
                    };
                }
                chart.into_any_element()
            }
            "RadarChart" => {
                let mut chart = RadarChart::new(data)
                    .value(|row| row.value)
                    .label(|row| row.label.clone());
                for op in &self.ops {
                    chart = match op {
                        Op::Grid(value) => chart.grid(*value),
                        Op::Dot => chart.dot(),
                        Op::GridLevels(value) => chart.grid_levels(*value),
                        _ => chart,
                    };
                }
                chart.into_any_element()
            }
            _ => unreachable!(),
        }
    }
}

struct ChartMaterializer(&'static str);
impl ComponentMaterializer for ChartMaterializer {
    fn materialize(&self, request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let argument = request
            .payload()
            .downcast_ref::<Payload>()
            .ok_or_else(|| anyhow::anyhow!("{} received an incompatible payload", self.0))?
            .0
            .clone();
        let callback = request.resolve_data_callback(&argument)?;
        let ops = request
            .methods()
            .filter_map(|m| m.payload().downcast_ref::<Op>().copied())
            .collect();
        wrap(
            request,
            ChartHost {
                kind: self.0,
                callback,
                ops,
            },
        )
    }
}

fn positive_usize_method(name: &'static str, op: fn(usize) -> Op) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new(name, ArgumentSchema::Number)],
        move |args| match args {
            [ComponentArgument::Number(value)]
                if value.is_finite()
                    && *value >= 1.
                    && value.fract() == 0.
                    && *value <= usize::MAX as f64 =>
            {
                Ok(ComponentPayload::new(op(*value as usize)))
            }
            _ => Err(format!("{name} expects a positive integer")),
        },
    )
    .with_documentation("Configures this chart option with a positive integer.")
}
fn number_method(name: &'static str, op: fn(f32) -> Op) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new(name, ArgumentSchema::Number)],
        move |args| match args {
            [ComponentArgument::Number(value)]
                if value.is_finite() && *value >= 0. && *value <= f32::MAX as f64 =>
            {
                Ok(ComponentPayload::new(op(*value as f32)))
            }
            _ => Err(format!("{name} expects a non-negative finite number")),
        },
    )
    .with_documentation("Configures this chart option with a non-negative number.")
}
fn flag(name: &'static str, op: Op) -> MethodDescriptor {
    MethodDescriptor::new(name, vec![], move |_| Ok(ComponentPayload::new(op)))
        .with_documentation("Selects this chart rendering mode.")
}

fn descriptor(name: &'static str, methods: Vec<MethodDescriptor>) -> ComponentDescriptor {
    ComponentDescriptor::new(name, Arc::new(ChartMaterializer(name)))
.with_constructors(vec![ConstructorDescriptor::new(
            name,
            vec![ArgumentDescriptor::new(
                "rows",
                ArgumentSchema::Callback(
                    "(cx: Context) => readonly { label: string; value: number }[]",
                ),
            )],
            |args| match args {
                [argument @ ComponentArgument::Callback(_)] => {
                    Ok(ComponentPayload::new(Payload(argument.clone())))
                }
                _ => Err("chart expects a row snapshot callback".into()),
            },
        )])
.with_methods(methods)
.with_documentation(
            "A concrete native chart backed by one immutable plain-data snapshot per materialization. Rows require { label, value }; style applies to its full-size host.",
        )
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(descriptor(
        "BarChart",
        vec![
            bool_method("Chart", "grid", "Configures this chart option.", Op::Grid),
            bool_method(
                "Chart",
                "label_axis",
                "Configures this chart option.",
                Op::Axis,
            ),
            bool_method(
                "Chart",
                "value_axis",
                "Configures this chart option.",
                Op::ValueAxis,
            ),
            positive_usize_method("tick_margin", Op::TickMargin),
        ],
    ))?;
    registry.register(descriptor(
        "LineChart",
        vec![
            bool_method("Chart", "grid", "Configures this chart option.", Op::Grid),
            bool_method("Chart", "x_axis", "Configures this chart option.", Op::Axis),
            positive_usize_method("tick_margin", Op::TickMargin),
            flag("dot", Op::Dot),
            flag("natural", Op::Natural),
            flag("linear", Op::Linear),
            flag("step_after", Op::StepAfter),
        ],
    ))?;
    registry.register(descriptor(
        "AreaChart",
        vec![
            bool_method("Chart", "grid", "Configures this chart option.", Op::Grid),
            bool_method("Chart", "x_axis", "Configures this chart option.", Op::Axis),
            positive_usize_method("tick_margin", Op::TickMargin),
            flag("natural", Op::Natural),
            flag("linear", Op::Linear),
            flag("step_after", Op::StepAfter),
        ],
    ))?;
    registry.register(descriptor(
        "PieChart",
        vec![
            number_method("inner_radius", Op::InnerRadius),
            number_method("pad_angle", Op::PadAngle),
            bool_method(
                "Chart",
                "labels",
                "Configures this chart option.",
                Op::Labels,
            ),
        ],
    ))?;
    registry.register(descriptor(
        "RadarChart",
        vec![
            bool_method("Chart", "grid", "Configures this chart option.", Op::Grid),
            positive_usize_method("grid_levels", Op::GridLevels),
            flag("dot", Op::Dot),
        ],
    ))?;
    Ok(())
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) mod test_probe {
    use std::cell::RefCell;
    thread_local! { static ERROR: RefCell<Option<String>> = const { RefCell::new(None) }; }
    pub(super) fn error(error: String) {
        ERROR.with(|slot| *slot.borrow_mut() = Some(error));
    }
    pub(crate) fn take_error() -> Option<String> {
        ERROR.with(|slot| slot.borrow_mut().take())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_contains_only_concrete_constructible_charts() {
        let mut registry = ComponentRegistry::new(
            gpui_shell::COMPONENT_REGISTRY_API_VERSION,
            gpui_shell::DEFAULT_COMPONENT_MODULE,
        )
        .unwrap();
        register(&mut registry).unwrap();
        assert_eq!(
            registry
                .freeze()
                .unwrap()
                .descriptors()
                .map(|d| d.name())
                .collect::<Vec<_>>(),
            [
                "BarChart",
                "LineChart",
                "AreaChart",
                "PieChart",
                "RadarChart"
            ]
        );
    }
}
