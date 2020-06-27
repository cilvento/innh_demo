use clap::{App,Arg};
use std::str::FromStr; 
use serde::{Deserialize, Serialize};
use csv::Reader;
use csv::Writer;
use b2dp::{Eta,GeneratorOpenSSL,utilities::bounds::PartitionBound,utilities::bounds::PartitionBoundOptions};
use b2dp::mechanisms::integerpartition::{IntegerPartitionOptions,integer_partition_mechanism_with_weights};
use b2dp::utilities::weights::WeightTable;
use b2dp::{exponential_mechanism,ExponentialOptions};

#[derive(Deserialize,Debug,Clone)]
struct Record {
    name: String,
    count: u64,
}

#[derive(Deserialize,Serialize,Debug)]
struct AttributedRecord {
    name: String,
    count: u64,
    ideal_partition: i64,
    attributed: i64,
}
#[derive(Deserialize,Debug)]
struct BoundRecord {
    name: String,
    lower: i64,
    upper: i64,
    estimate: i64,
}

enum PartitionStrategies {
    Laplace,
    HistoricalDistance,
    //HistoricalTrend,
    FromFile,
    Naive,
}

impl FromStr for PartitionStrategies {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Laplace" => Ok(PartitionStrategies::Laplace),
            "HistoricalDistance" => Ok(PartitionStrategies::HistoricalDistance),
            "FromFile" => Ok(PartitionStrategies::FromFile),
            "Naive" => Ok(PartitionStrategies::Naive),
            _ => Err("No matching partition strategy")
        }
    }
}


fn main() -> Result<(), &'static str> {
    
    let matches = App::new("Integer Histogram Demo")
        .author("Christina Ilvento <cilvento@gmail.com>")
        .about("Does stuff")
        .arg(Arg::with_name("INPUT")
            .about("Sets the private input file.")
            .value_name("INPUT")
            .required(true)
            .index(1))
        .arg(Arg::from("[bounds_strategy] 'The type to use'")
            .possible_values(&["Laplace", "HistoricalDistance","FromFile","Naive"])
            .short('b')
            .long("bounds")
            .about("Strategy for partition bounds generation.")
            .takes_value(true)
            .required(true)
            )
        .arg(Arg::from("[attribution_strategy] 'The type to use'")
            .possible_values(&["Basic", "Scoped"])
            .short('a')
            .long("attr")
            .about("Strategy for attribution.")
            .takes_value(true)
            )
        .arg(Arg::with_name("HISTORICAL")
            .about("Sets the file name for historical data")
            .short('h')    
            .long("historical")
            .takes_value(true)
            .required_if("bounds_strategy","HistoricalDistance")
        )
        .arg(Arg::with_name("BOUNDS")
            .about("Sets the input file name for predetermined bounds")
            .short('f')    
            .long("bfile")
            .takes_value(true)
            .required_if("bounds_strategy","FromFile")
        )
        .arg(Arg::with_name("NUM_TRIALS")
            .about("The number of trial loops to run.")    
            .short('t')
            .long("trials")
            .takes_value(true)
        )
        .arg(Arg::with_name("SPARSITY_CONTROL")
            .about("Whether to use sparsity control.")    
            .short('s')
            .long("sparsity")
            .takes_value(true)
        )
        .get_matches();

    let input_file = matches.value_of("INPUT").unwrap();
    println!("Private data input: {:?}", input_file);
    
    let bounds_strategy = matches.value_of("bounds_strategy").unwrap_or("Naive");
    println!("Bounds generation Strategy: {:?}", bounds_strategy);
    let num_trials: u32 = matches.value_of_t("NUM_TRIALS").unwrap_or(1); 
    let sparsity_control: bool = matches.value_of_t("SPARSITY_CONTROL").unwrap_or(false);
    println!("Sparsity Control: {:?}", sparsity_control);

    let attr_strategy = matches.value_of("attribution_strategy").unwrap_or("Basic");
    println!("Attribution Strategy: {:?}", attr_strategy);

    // Preset privacy budgets
    let weight_budget = Eta::new(1,1,1)?;
    let pb_budget = Eta::new(7,3,1)?;
    let sparsity_budget = Eta::new(7,3,1)?;
    let ref_budget = Eta::new(7,3,1)?;
    let attribution_budget = Eta::new(1,1,1)?;

    // Read the input file as a set of records
    let mut reader = Reader::from_path(input_file).unwrap();
    let mut records: Vec<Record> = Vec::new();
    for record in reader.deserialize() {
        let record: Record = record.unwrap_or(Record {name: String::from(" "), count: 0}); 
        records.push(record); 
    }

    // Get the integer partition from the records
    let mut partition: Vec<i64> = records.iter().map(|r| r.count as i64 ).collect();
    partition.sort();
    partition.reverse();
    //println!("{:?}", records);
    //println!("{:?}", partition);

    let total_count: i64 = partition.iter().sum(); // TODO: take total_count as input
    let total_cells: usize = partition.len(); // TODO: take as argument 

    // Read in the bound file
    let bounds_file = matches.value_of("BOUNDS");
    let mut boundrecords: Vec<BoundRecord> = Vec::new();
    if bounds_file.is_some() {
        println!("Bound source: {:?}", bounds_file.unwrap());
        let mut boundreader = Reader::from_path(bounds_file.unwrap()).unwrap();
        
        for record in boundreader.deserialize() {
            let record: BoundRecord = record.unwrap_or(BoundRecord {name: String::from(" "), lower: 0, upper:0, estimate:0}); 
            boundrecords.push(record); 
        }
    }
    // Read in the Historical file
    let hist_file = matches.value_of("HISTORICAL");
    let mut histrecords: Vec<Record> = Vec::new();
    if hist_file.is_some() {
        println!("Historical source: {:?}", hist_file.unwrap());
        let mut histreader = Reader::from_path(hist_file.unwrap()).unwrap();
        
        for record in histreader.deserialize() {
            let record: Record = record.unwrap_or(Record {name: String::from(" "), count: 0}); 
            histrecords.push(record); 
        }
    }

    // Get the partition bounds
    let pb = match bounds_strategy {
        "Laplace" => {

                        let rng = GeneratorOpenSSL {};
                        let mut pb_options: PartitionBoundOptions = Default::default();
                        if sparsity_control {pb_options.sparsity_control = Some(sparsity_budget); }
                        PartitionBound::from_noisy_estimates(total_count as usize, 
                                                             Some(total_cells),  // TODO: use total cells depending on args
                                                             &partition, 
                                                             pb_budget, 
                                                             rng, 
                                                             pb_options)?
                     },
        "HistoricalDistance" => {
            let mut histpartition: Vec<i64> = histrecords.iter().map(|r| r.count as i64 ).collect();
            histpartition.sort();
            histpartition.reverse(); 
            PartitionBound::with_reference( total_count as usize, 
                                            &histpartition, 
                                            &partition,
                                            ref_budget)?
         },
         "FromFile" => {
            let mut lower: Vec<i64> = boundrecords.iter().map(|r| r.lower as i64 ).collect();
            lower.sort();
            lower.reverse();
            let mut upper: Vec<i64> = boundrecords.iter().map(|r| r.upper as i64 ).collect();
            upper.sort();
            upper.reverse();
            let mut estimate: Vec<i64> = boundrecords.iter().map(|r| r.estimate as i64 ).collect();
            estimate.sort();
            estimate.reverse();
            let float_estimates: Vec<f64> = estimate.iter().map(|r| *r as f64).collect();
            // Add terminating zeros
            lower.push(0);
            upper.push(0);
            PartitionBound { upper,
                             lower,
                             count: total_count as usize, 
                             cells: boundrecords.len(), 
                             sparsity_control: false, 
                             noisy_estimates: Some(float_estimates) }
            
            
         } , 
        _ => PartitionBound::new(total_count as usize)? // Default is Naive
    };

    // // Get the weight table
    let mut weight_table = WeightTable::from_bounds(weight_budget, &pb, &partition)?;

    // Get the bias
    let bias = weight_table.get_bias(&pb,&partition)?;
    for b in bias.iter() {print!("{:?}, ",b);}
    println!();

    // Increase precision of weight_table
    let inc = weight_table.arithmetic_config.precision;
    weight_table.arithmetic_config.increase_precision(inc)?;
    // Trial Loop
    for i in 0..num_trials {
        // Get the private partition
        let options: IntegerPartitionOptions = Default::default(); // TODO: Allow option specification
        let ip = integer_partition_mechanism_with_weights(& mut weight_table, &pb, options)?;

        // Reattribute 
        let mut attributed_records = match attr_strategy {
            "Scoped" => attribute_scoped(attribution_budget, &ip, & mut records, & mut boundrecords, total_count)?,
            _ => attribute(attribution_budget, &ip, & mut records, total_count)?
        };
        
        // Output
        // Sort by canonical ordering:  true count and then name
        attributed_records.sort_by(|r1, r2| r1.count.cmp(&r2.count).reverse().then(r1.name.cmp(&r2.name)));     
        
        let counts: Vec<u64> = attributed_records.iter().map(|r| r.count).collect();
        
        //writer.serialize(&counts);
        for b in counts.iter() {print!("{:?}, ",b);}
        println!();
        
        let ideals: Vec<i64> = attributed_records.iter().map(|r| r.ideal_partition).collect();
        for b in ideals.iter() {print!("{:?}, ",b);}
        println!();
        let attr: Vec<i64> = attributed_records.iter().map(|r| r.attributed).collect();
        
        for b in attr.iter() {print!("{:?}, ",b);}
        println!();
     }

    Ok(())
}



/// Scoped reattribution
fn attribute_scoped(eta: Eta, 
                    ip: & Vec<i64>,  
                    records: & mut Vec<Record>, 
                    boundrecords: & mut Vec<BoundRecord>,  
                    total_count: i64) 
    -> Result<Vec<AttributedRecord>, &'static str> 
{
    // sort the records alphabetically (this ordering is independent of the values of the records.)
    records.sort_by(|r1, r2| r1.name.cmp(&r2.name));
    // We assume that the names in records and boundrecords are the same
    boundrecords.sort_by(|r1, r2| r1.name.cmp(&r2.name));
    let mut attributed_records: Vec<AttributedRecord> = Vec::new();
    // iterate through the records
    for i in 0..records.len() {
        let r = &records[i];
        let options: ExponentialOptions =  Default::default(); //  TODO:  change this to Use optimized sampling. 
        let rng = GeneratorOpenSSL {};
        
        // Construct the outcome space
        let outcomes: Vec<i64> = (boundrecords[i].lower..boundrecords[i].upper + 1).collect();
        
        let utility_min = 0;
        let utility_max = *ip.iter().max().unwrap_or(&total_count); // Note: introduces a timing channel
        
        // construct utility function
        let basic_utility = |x: &i64| (*x - r.count as i64).abs() as f64;
        // select  a value from the integer  partition
        let s = exponential_mechanism(eta, &outcomes, basic_utility, utility_min, utility_max, outcomes.len() as u32, rng, options)?;
        // create a new attributed record 
        let attributed_record = AttributedRecord { name: String::from(&r.name), count: r.count, ideal_partition: 0, attributed: *s};
        attributed_records.push(attributed_record);
    }

    // Sort attributed records by attributed value, then name
    attributed_records.sort_by(|r1, r2| r1.attributed.cmp(&r2.attributed).reverse().then(r1.name.cmp(&r2.name)));     
    

    // Assign and Tie Break by name
    for i in 0..attributed_records.len() {
        attributed_records[i].attributed = ip[i];
    }
    
    // Assign ideal partition values
    attributed_records.sort_by(|r1, r2| r1.count.cmp(&r2.count).reverse());
    for i in 0..attributed_records.len() {
        attributed_records[i].ideal_partition = ip[i];
    }

    Ok(attributed_records)
}

/// Basic reattribution
fn attribute(eta: Eta, ip: & Vec<i64>,  records: & mut Vec<Record>, total_count: i64) -> Result<Vec<AttributedRecord>, &'static str> 
{
    // sort the records alphabetically (this ordering is independent of the values of the records.)
    records.sort_by(|r1, r2| r1.name.cmp(&r2.name));
    let mut attributed_records: Vec<AttributedRecord> = Vec::new();
    // iterate through the records
    for r in records {
        let options: ExponentialOptions =  Default::default(); //  TODO:  change this to Use optimized sampling. 
        let rng = GeneratorOpenSSL {};
        let utility_min = 0;
        let utility_max = *ip.iter().max().unwrap_or(&total_count); // Note: introduces a timing channel
        
        // construct utility function
        let basic_utility = |x: &i64| (*x - r.count as i64).abs() as f64;
        // select  a value from the integer  partition
        let s = exponential_mechanism(eta, ip, basic_utility, utility_min, utility_max,ip.len() as u32, rng, options)?;
        // create a new attributed record 
        let attributed_record = AttributedRecord { name: String::from(&r.name), count: r.count, ideal_partition: 0, attributed: *s};
        attributed_records.push(attributed_record);
    }

    // Sort attributed records by attributed value, then name
    attributed_records.sort_by(|r1, r2| r1.attributed.cmp(&r2.attributed).reverse().then(r1.name.cmp(&r2.name)));     
    

    // Assign and Tie Break by name
    for i in 0..attributed_records.len() {
        attributed_records[i].attributed = ip[i];
    }
    
    // Assign ideal partition values
    attributed_records.sort_by(|r1, r2| r1.count.cmp(&r2.count).reverse());
    for i in 0..attributed_records.len() {
        attributed_records[i].ideal_partition = ip[i];
    }

    Ok(attributed_records)
}
