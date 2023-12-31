use std::thread;
use rand::distributions::Uniform;
use rand::distributions::{Distribution, Standard};
use rand::{thread_rng, Rng};
use statrs::distribution::{Normal, Poisson, StudentsT, Triangular, Weibull};
extern crate rayon;
use rayon::prelude::*;

extern crate serde;
extern crate serde_json;
use serde::Deserializer;
use serde::{Deserialize, Serialize};
use serde_json::json;

// use std::error::Error;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Read;
use std::io::Write;
use std::time::{Duration, Instant};
use std::{fs, io, process};
use std::error::Error;

use csv::Writer;


pub mod limits{
    pub fn min(a:f64,b:f64)->f64{
        if a<b{
            a
        }
        else{
            b
        }
    }
    pub fn max(a:f64,b:f64)->f64{
        if a<b{
            b
        }
        else{
            a
        }
    }
}

pub fn poisson(rate:f64)->u64{
    let mut rng = thread_rng();
    let v: &Vec<f64> = &Poisson::new(rate)
    .unwrap()
    .sample_iter(&mut rng)
    .take(1)
    .collect();     
    v[0] as u64
}

pub fn normal(mean: f64, std: f64, upper:f64) -> f64 {
    let mut thing: f64 = 0.0;
    loop {
        // println!("Mean value is {}, STD value is {}",mean,std);
        let mut rng = thread_rng();
        let v: &Vec<f64> = &Normal::new(limits::max(0.0001,mean), limits::max(0.0001,std))
            .unwrap()
            .sample_iter(&mut rng)
            .take(1)
            .collect();
        thing = v[0];
        if thing > 0.0 && thing<upper{
            break;
        }
    }
    thing
}

pub fn roll(prob:f64)->bool{
    let mut rng = thread_rng();
    let roll = Uniform::new(0.0, 1.0);
    let rollnumber: f64 = rng.sample(roll);
    rollnumber<prob    
}

pub fn uniform(start:f64,end:f64)->f64{
    let mut rng = thread_rng();
    let roll = Uniform::new(start, end);
    let rollnumber: f64 = rng.sample(roll);
    rollnumber
}

#[derive(Clone)]
pub struct Zone_3D{
    segments:Vec<Segment_3D>, 
    zone:usize,
    capacity:u32,
    eviscerate:bool
}

#[derive(Clone)]
pub struct Segment_3D{
    zone:usize,
    origin_x:u64,
    origin_y:u64,
    origin_z:u64,
    range_x:u64,
    range_y:u64,
    range_z:u64,
    capacity:u32,
    eviscerated:bool
}

pub struct Eviscerator{
    zone:usize,
    infected: bool,
    count_since_infected:u8
}

impl Zone_3D{
    fn add(&mut self)->[u64;7]{
        // println!("Adding to {}",self.zone);
        let mut origin_x:u64 = 0;
        let mut origin_y:u64 = 0;
        let mut origin_z:u64 = 0;
        let mut range_x:u64 = 0;
        let mut range_y:u64 = 0;
        let mut range_z:u64 = 0;
        if let Some(first_space) = self.segments.iter_mut().find(|item| item.capacity >= 1){ //0 occupants 
            //Segment capacity update
            first_space.capacity -= 1; //add 1 occupant
            origin_x = first_space.origin_x;
            origin_y = first_space.origin_y;
            origin_z = first_space.origin_z;
            range_x = first_space.range_x;
            range_y = first_space.range_y;
            range_z = first_space.range_z;
        }        
        let condition:bool = origin_x != 0 && origin_y != 0 && origin_z != 0;
        if condition{
            self.capacity-=1;
        }
        [condition as u64,origin_x,origin_y,origin_z,range_x,range_y,range_z]
    }
    fn subtract(&mut self,x:u64,y:u64,z:u64){
        self.capacity+=1;
        self.segments.iter_mut().filter_map(|mut seg| {
            if x == seg.origin_x && y == seg.origin_y && z == seg.origin_z{
                seg.capacity += 1;
            }
            Some(seg)
        });
    }
    fn generate_empty(zone:usize,grid:[u64;3],step:[usize;3])->Zone_3D{
        let mut vector:Vec<Segment_3D> = Vec::new();
        for x in (0..grid[0]).step_by(step[0]){
            for y in (0..grid[1]).step_by(step[1]){
                for z in (0..grid[2]).step_by(step[2]){
                    vector.push(Segment_3D{zone:zone.clone(),origin_x:x,origin_y:y,origin_z:z,range_x:step.clone()[0] as u64,range_y:step.clone()[1] as u64,range_z:step.clone()[2] as u64,capacity:NO_OF_HOSTS_PER_SEGMENT[zone] as u32, eviscerated:false})
                }
            }
        }
        Zone_3D{segments:vector,zone:zone, capacity:(grid[0] as u32)*(grid[1] as u32)*(grid[2] as u32)/ ((step[0]*step[1]*step[2]) as u32)*NO_OF_HOSTS_PER_SEGMENT[zone] as u32,eviscerate:EVISCERATE_ZONES.contains(&zone)}
    }
    fn generate_full(zone:usize,grid:[u64;2],step:[usize;3])->Zone_3D{
        let mut vector:Vec<Segment_3D> = Vec::new();
        for x in (0..grid[0]).step_by(step[0]){
            for y in (0..grid[1]).step_by(step[1]){
                for z in (0..grid[2]).step_by(step[2]){
                    vector.push(Segment_3D{zone:zone.clone(),origin_x:x,origin_y:y,origin_z:z,range_x:step.clone()[0] as u64,range_y:step.clone()[1] as u64,range_z:step.clone()[2] as u64,capacity:0,eviscerated:false})
                }
            }
        }
        Zone_3D{segments:vector,zone:zone, capacity:0,eviscerate:EVISCERATE_ZONES.contains(&zone)}
    }    
    fn feed_setup(self, vector:Vec<host>, time:usize)->Vec<host>{
        let mut vec:Vec<host> = vector.clone();
        for segment in self.clone().segments{
            host::feed(&mut vec,segment.origin_x.clone(),segment.origin_y.clone(),segment.origin_z.clone(),segment.zone.clone(),time);
        }
        vec
    }
    fn eviscerate(&mut self,eviscerators:&mut Vec<Eviscerator>, vector:&mut Vec<host>,time:usize){
        //Filter out eviscerators that are for the zone in particular
        let mut filtered_vector: Vec<&mut Eviscerator> = eviscerators.iter_mut().filter(|ev| ev.zone == self.zone).collect();
        // Define the step size for comparison
        let step_size = filtered_vector.len();

        // Iterate over the smaller vector and compare with elements in the larger vector at regular intervals
        for (i, eviscerator) in filtered_vector.iter_mut().enumerate() {
            let start_index = i; // Start index in the larger vector for this eviscerator
            for (j, host) in vector.iter_mut().skip(start_index).step_by(step_size).enumerate() {
                // Compare and update the elements in the larger vector
                // if eviscerator.values_are_greater(larger_value) {
                //     *larger_value = eviscerator.values.clone(); // Assuming your struct has a clone method
                if host.infected && host.zone == eviscerator.zone{
                    eviscerator.infected = true;
                    // println!("EVISCERATOR HAS BEEN INFECTED AT TIME {} of this chicken stock entering zone!",host.time);
                    eviscerator.count_since_infected = 0;
                    println!("{} {} {} {} {} {}",host.x,host.y,host.z,12,time,host.zone);
                }else if eviscerator.infected && host.zone == eviscerator.zone{
                    // println!("Confirming that an eviscerator is infected in zone {}",eviscerator.zone);
                    eviscerator.count_since_infected += 1;
                    host.infected = host.transfer();
                    if host.infected{
                        println!("{} {} {} {} {} {}",host.x,host.y,host.z,11,time,host.zone);
                        // panic!("Evisceration has infected a host!!!");
                    }
                }
                //Decay of infection
                if eviscerator.count_since_infected>=EVISCERATE_DECAY{
                    eviscerator.infected = false;
                }
            }
        }
        
    }
}

#[derive(Clone)]
pub struct host{
    infected:bool,
    motile:u8,
    zone:usize, //Possible zones denoted by ordinal number sequence
    prob1:f64,  //Probability of contracting disease - these are tied to zone if you create using .new() implementation within methods
    prob2:f64,  //standard deviation if required OR second probabiity value for transferring in case that is different from prob1
    x:f64,
    y:f64,
    z:f64, //can be 0 if there is no verticality
    age:f64,  //Age of host
    time:f64, //Time chicken has spent in facility - start from 0.0 from zone 0
    origin_x:u64,
    origin_y:u64,
    origin_z:u64,
    restrict:bool,  //Are the hosts free roaming or not?
    range_x:u64,  //"Internal" GRIDSIZE to simulate caged chickens in side the zone itself, not free roaming within facility ->Now to be taken from Segment
    range_y:u64,  //Same as above but for the y direction
    range_z:u64
}
//Note that if you want to adjust the number of zones, you have to, in addition to adjusting the individual values to your liking per zone, also need to change the slice types below!
//Space
const LISTOFPROBABILITIES:[f64;3] = [0.8,0.5,0.5]; //Probability of transfer of samonella per zone - starting from zone 0 onwards
const GRIDSIZE:[[f64;3];3] = [[100.0,50.0,8.0],[100.0,100.0,20.0],[4500.0,2.0,2.0]];
const MAX_MOVE:f64 = 3.0;
const MEAN_MOVE:f64 = 2.0;
const STD_MOVE:f64 = 1.0; // separate movements for Z config
const MAX_MOVE_Z:f64 = 1.0;
const MEAN_MOVE_Z:f64 = 2.0;
const STD_MOVE_Z:f64 = 4.0;
const NO_OF_HOSTS_PER_SEGMENT:[u8;3] = [10,3,1];
//Space --- Segment ID
const TRANSFERS_ONLY_WITHIN:bool = false; //Boolean that informs simulation to only allow transmissions to occur WITHIN segments, not between adjacent segments
//Fly option
const FLY:bool = false;
const FLY_FREQ:u8 = 3; //At which Hour step do the  
//Disease 
const TRANSFER_DISTANCE: f64 = 0.7;//maximum distance over which hosts can trasmit diseases to one another
//Host parameters
const PROBABILITY_OF_INFECTION:f64 = 0.12; //probability of imported host being infected
const MEAN_AGE:f64 = 5.0*24.0; //Mean age of hosts imported (IN HOURS)
const STD_AGE:f64 = 3.0*24.0;//Standard deviation of host age (when using normal distribution)
const MAX_AGE:f64 = 11.0*24.0; //Maximum age of host accepted (Note: as of now, minimum age is 0.0)
const DEFECATION_RATE:f64 = 6.0; //Number times a day host is expected to defecate
const DEPOSIT_RATE:f64 = 0.00001; //Number of times a day host is expected to deposit a consumable deposit
//Feed parameters
const FEED:bool = true; //Do the hosts get fed?
const FEED_INFECTION_RATE:f64 = 0.003; //Probability of feed being infected
const FEED_ZONES:[usize;1] = [1]; //To set the zones that have feed provided to them.
const FEED_TIMES: [usize;2] = [11,14]; //24h format, when hosts get fed: Does not have to be only 2 - has no link to number of zones or anything like that
//Evisceration parameters
const EVISCERATE:bool = true;
const EVISCERATE_ZONES:[usize;1] = [2]; //Zone in which evisceration takes place
const EVISCERATE_DECAY:u8 = 5;
const NO_OF_EVISCERATORS:[usize;1] = [6];

//Transfer parameters
const ages:[f64;3] = [8.0,1.0,1.0]; //Time hosts are expected spend in each region minimally
//Collection
const AGE_OF_HOSTCOLLECTION: f64 = 20.0*24.0;  //For instance if you were collecting chickens every 15 days
const COLLECT_DEPOSITS: bool = false;
const AGE_OF_DEPOSITCOLLECTION:f64 = 1.0*24.0; //If you were collecting their eggs every 3 days
const FAECAL_CLEANUP_FREQUENCY:usize = 2; //How many times a day do you want faecal matter to be cleaned up?
//or do we do time collection instead?
const TIME_OF_COLLECTION :f64 = 1.0; //Time that the host has spent in the last zone from which you collect ONLY. NOT THE TOTAL TIME SPENT IN SIMULATION
//Resolution
const STEP:[[usize;3];3] = [[4,4,2],[5,5,2],[2,2,1]];  //Unit distance of segments ->Could be used to make homogeneous zoning (Might not be very flexible a modelling decision)
const HOUR_STEP: f64 = 2.0; //Number of times hosts move per hour
const LENGTH: usize = 24; //How long do you want the simulation to be?
//Influx? Do you want new chickens being fed into the scenario everytime the first zone exports some to the succeeding zones?
const INFLUX:bool = true;
const PERIOD_OF_INFLUX:u8 = 24; //How many hours before new batch of hosts are imported?
const PERIOD_OF_TRANSPORT:u8 = 1; //Prompt to transport chickens between zones every hour (checking that they fulfill ages requirement of course)
//Restriction?
const RESTRICTION:bool = true;
//Generation Parameters
const SPORADICITY:f64 = 4.0; //How many fractions of the dimension of the cage/segment do you want the hosts to start at? Bigger number makes the spread of hosts starting point more even per seg


//Additional 3D parameters
const FAECAL_DROP:bool = true; //Does faeces potentially drop in terms of depth?
const PROBABILITY_OF_FAECAL_DROP:f64 = 0.3;




impl host{
    fn feed(mut vector:&mut Vec<host>, origin_x:u64,origin_y:u64,origin_z:u64, zone:usize,time:usize){
        if roll(FEED_INFECTION_RATE){
            // println!("Infected feed confirmed");
            vector.iter_mut().for_each(|mut h|{
                if h.motile == 0 && !h.infected && h.origin_x == origin_x && h.origin_y == origin_y && h.origin_z == origin_z && h.zone == zone{
                    h.infected = h.transfer();
                    println!("{} {} {} {} {} {}",h.x,h.y,h.z,10,time,h.zone); //10 is now an interaction type driven by the infected feed
                }
            })
        }
    }
    fn infect(mut vector:Vec<host>,loc_x:u64,loc_y:u64,loc_z:u64,zone:usize)->Vec<host>{
        if let Some(first_host) = vector.iter_mut().filter(|host_| host_.zone == zone).min_by_key(|host| {
            let dx = host.origin_x as i64 - loc_x as i64;
            let dy = host.origin_y as i64 - loc_y as i64;
            let dz = host.origin_z as i64 - loc_z as i64;
            (dx*dx + dy*dy+dz*dz) as u64
        }) 
        {if !first_host.infected{first_host.infected=true;}}
        vector
    }
    fn infect_multiple(mut vector:Vec<host>,loc_x:u64,loc_y:u64,loc_z:u64,n:usize,zone:usize)->Vec<host>{ //homogeneous application ->Periodically apply across space provided,->Once per location
        let mut filtered_vector: Vec<&mut host> = vector.iter_mut().filter(|host| host.zone == zone).collect();

        filtered_vector.sort_by_key(|host| {
            let dx = host.origin_x as i64 - loc_x as i64;
            let dy = host.origin_y as i64 - loc_y as i64;
            let dz = host.origin_z as i64 - loc_z as i64;
            (dx*dx + dy*dy+dz*dz) as u64
        }) ;
        for host in filtered_vector.iter_mut().take(n){
            host.infected = true;
            println!("{} {} {} {} {} {}",host.x,host.y,host.z,0,0.0,host.zone);
        }
        vector
    }
    fn transport(mut vector:&mut Vec<host>,space:&mut Vec<Zone_3D>, influx: bool){ //Also to change ;size if you change number of zones
        let mut output:Vec<host> = Vec::new();
        for zone in (0..space.len()).rev(){
            let mut __:u32 = space.clone()[zone].capacity;
            if &space[zone].capacity>&0 && zone>0{ //If succeeding zones (obviously zone 0 doesn't have do to this - that needs to be done with a replace mechanism)
                let zone_toedit:&mut Zone_3D = &mut space[zone];
                vector.iter_mut().for_each(|mut x| {
                    if x.zone == zone-1 && x.time>ages[zone-1] && __>0 && x.motile == 0 && space[zone].capacity>0{ //Hosts in previous zone that have spent enough time spent in previous zone
                        //Find the first available segment
                        // println!("Transporting...");
                        __ -= 1;
                        space[zone-1].subtract(x.origin_x.clone(),x.origin_y.clone(),x.origin_z.clone()); //move host from previous zone
                        // println!("{} capacity for zone {} vs {} for zone {}", &space[zone-1].capacity, zone-1,&space[zone].capacity,zone);
                        x.zone += 1;
                        // println!("Moved to zone {}",x.zone);
                        x.time = 0.0;
                        x.prob1 = LISTOFPROBABILITIES[x.zone];
                        // println!("Going to deduct capacity @  zone {} with a capacity of {}", zone,zone_toedit.clone().zone);
                        // println!("Apparently think that zone {} has {} space left",zone,space[zone].capacity);
                        let vars:[u64;7] =  space[zone].add();
                        if vars[0] != 0{
                            x.origin_x = vars[1];
                            x.origin_y = vars[2];
                            x.origin_z = vars[3];
                            x.range_x = vars[4];
                            x.range_y = vars[5];
                            x.range_z = vars[6];

                            //Maybe try moving the chickens randomly within each new section otherwise they all will infect each other at origin
                            let mean_x:f64 = ((x.range_x as f64)/2.0) as f64;
                            let std_x:f64 = ((x.range_x as f64)/SPORADICITY) as f64;
                            let max_x:f64 = x.range_x as f64;
                            let mean_y:f64 = ((x.range_y as f64)/2.0) as f64;
                            let std_y:f64 = ((x.range_y as f64)/SPORADICITY) as f64;
                            let max_y:f64 = x.range_y as f64;              
                            //Baseline starting point in new region
                            x.x = normal(mean_x,std_x,(x.origin_x+x.range_x) as f64);
                            x.y = normal(mean_y,std_x,(x.origin_y+x.range_y) as f64);
                            x.z = x.origin_z as f64;
                        }
                    }
                })
            // output.append(&mut vector);
            }
            else if zone == 0 && space[zone].capacity>0 && influx{ //replace mechanism : influx is determined by INFLUX and PERIOD OF INLFUX 
                // let mut zone_0:&mut Zone = &mut space[0];
                for _ in 0..space[zone].clone().capacity as usize{
                    // let [x,y]:[u64;2] = space[zone].add();
                    //Roll probability
                    let mut rng = thread_rng();
                    let roll = Uniform::new(0.0, 1.0);
                    let rollnumber: f64 = rng.sample(roll);
                    let [condition,x,y,z,range_x,range_y,range_z] = space[0].add(); 
                    if rollnumber<PROBABILITY_OF_INFECTION && condition != 0{
                        vector.push(host::new_inf(0,0.2,x as f64,y as f64,z as f64,RESTRICTION,range_x,range_y,range_z));
                    }
                    else if condition != 0{
                        vector.push(host::new(0,0.2,x as f64,y as f64,z as f64,RESTRICTION,range_x,range_y,range_z));
                    }
            }
        }
    }
}



    fn transfer(&self)->bool{ //using prob1 as the probability of contracting disease  (in other words, no separation of events between transferring and capturing disease. If something is infected, it is always infected. Potentially.... the prospective new host will not get infected, but the INFECTED is always viably transferring)
        let mut rng = thread_rng();
        let roll = Uniform::new(0.0, 1.0);
        let rollnumber: f64 = rng.sample(roll);
        // println!("DISEASE   {}",rollnumber);
        rollnumber < self.prob1
    }
    fn new(zone:usize, std:f64,loc_x:f64, loc_y:f64,loc_z:f64,restriction:bool,range_x:u64,range_y:u64,range_z:u64)->host{
        //We shall make it such that the chicken is spawned within the bottom left corner of each "restricted grid" - ie cage
        let prob:f64 = LISTOFPROBABILITIES[zone.clone()];
        //Add a random age generator
        host{infected:false,motile:0,zone:zone,prob1:prob,prob2:std,x:loc_x as f64,y:loc_y as f64,z:loc_z as f64,age:normal(MEAN_AGE,STD_AGE,MAX_AGE),time:0.0, origin_x:loc_x as u64,origin_y:loc_y as u64,origin_z: loc_z as u64,restrict:restriction,range_x:range_x,range_y:range_y,range_z:range_z}
    }
    fn new_inf(zone:usize, std:f64,loc_x:f64, loc_y:f64,loc_z:f64,restriction:bool,range_x:u64,range_y:u64,range_z:u64)->host{
        let prob:f64 = LISTOFPROBABILITIES[zone.clone()];
        host{infected:true,motile:0,zone:zone,prob1:prob,prob2:std,x:loc_x as f64,y:loc_y as f64,z:loc_z as f64,age:normal(MEAN_AGE,STD_AGE,MAX_AGE),time:0.0, origin_x:loc_x as u64,origin_y:loc_y as u64,origin_z: loc_z as u64,restrict:restriction,range_x:range_x,range_y:range_y,range_z:range_z}
    }
    fn deposit(self, consumable: bool)->host{ //Direct way to lay deposit from host. The function is 100% deterministic and layering a probability clause before this is typically expected
        let zone = self.zone.clone();
        let prob1 = self.prob1.clone();
        let prob2 = self.prob2.clone();
        let x = self.x.clone();
        let y = self.y.clone();
        let mut z = self.z.clone();
        if !RESTRICTION{ //If there are no containers holding the hosts (ie RESTRICTION), these hosts are keeping themselves above z = 0 by flying/floating etc, then deposits will necessary FALL to the floor ie z = 0
            z = 0.0;
        }
        let inf = self.infected.clone();
        let range_y = self.range_y.clone();
        let range_x = self.range_x.clone();
        let range_z = self.range_z.clone();
        let restriction = self.restrict.clone();
        let origin_x = self.origin_x.clone();
        let origin_y = self.origin_y.clone();
        let origin_z = self.origin_z.clone();
        // println!("EGG BEING LAID");
        if consumable{host{infected:inf,motile:1,zone:zone,prob1:prob1,prob2:prob2,x:x,y:y,z:z,age:0.0,time:0.0,origin_x:x as u64,origin_y:y as u64,origin_z:z as u64,restrict:restriction,range_x:range_x,range_y:range_y,range_z:range_z}}
        else{
            // println!("Pooping!");
            host{infected:inf,motile:2,zone:zone,prob1:prob1,prob2:prob2,x:x,y:y,z:z,age:0.0,time:0.0,origin_x:x as u64,origin_y:y as u64,origin_z:z as u64,restrict:restriction,range_x:range_x,range_y:range_y,range_z:range_z}
        }
    }
    fn deposit_all(vector:Vec<host>)->Vec<host>{
        //Below is an example whereby hosts deposit twice a day (fecal matter and laying eggs each once per day as an example)
        let mut vecc:Vec<host> = vector.clone();
        let mut vecc_into: Vec<host> = vector.clone().into_par_iter().filter(|x| x.motile==0).collect::<Vec<_>>(); //With this re are RETAINING the hosts and deposits within the original vector

        //.map wasn't working so we brute forced a loop
        for ele in vecc_into{
            let mut rng = thread_rng();
            let v: &Vec<f64> = &Poisson::new(DEPOSIT_RATE/24.0)
            .unwrap()
            .sample_iter(&mut rng)
            .take(1)
            .collect();            
            for _ in 0..v[0] as usize{
                vecc.push(ele.clone().deposit(true));//non consumable excrement once per day rate
            }
            let mut rng = thread_rng();
            let v: &Vec<f64> = &Poisson::new(DEFECATION_RATE/24.0)
            .unwrap()
            .sample_iter(&mut rng)
            .take(1)
            .collect();            
            for _ in 0..v[0] as usize{
                vecc.push(ele.clone().deposit(false));//non consumable excrement once per day rate
            }
        }
        vecc
    }
    fn land(vector:Vec<host>)->Vec<host>{
        vector.into_par_iter().filter_map(|mut x| {
            if RESTRICTION{
                x.z = x.origin_z as f64;
                Some(x)
            }else{
                x.z = 0.0;
                Some(x)
            }
        }).collect()
    }
    fn shuffle(mut self)->host{
        if self.motile==0 && EVISCERATE_ZONES.contains(&self.zone) == false{
            //Whether the movement is negative or positive
            let mut mult:[f64;3] = [0.0,0.0,0.0];
            for index in 0..mult.len(){
                if roll(0.33){
                    if roll(0.5){
                        mult[index] = 1.0;
                    }else{
                        mult[index] = -1.0;
                    }
                }
            }

            let mut new_x:f64 = self.origin_x.clone() as f64;
            let mut new_y:f64 = self.origin_y.clone() as f64;
            let mut new_z:f64 = self.origin_z.clone() as f64;
            //use truncated normal distribution (which has been forced to be normal) in order to change the values of x and y accordingly of the host - ie movement
            if self.restrict{
                // println!("We are in the restrict clause! {}", self.motile);
                // println!("Current shuffling parameter is {}", self.motile);
                new_x = limits::min(limits::max(self.origin_x as f64,self.x+mult[0]*normal(MEAN_MOVE,STD_MOVE,MAX_MOVE)),(self.origin_x as f64+self.range_x as f64));
                new_y = limits::min(limits::max(self.origin_y as f64,self.y+mult[1]*normal(MEAN_MOVE,STD_MOVE,MAX_MOVE)),(self.origin_y as f64+self.range_y as f64));
                if FLY{
                    new_z = limits::min(limits::max(self.origin_z as f64,self.z+mult[2]*normal(MEAN_MOVE_Z,STD_MOVE_Z,MAX_MOVE_Z)),(self.origin_z as f64+self.range_z as f64));
                }
            }else{
                new_x = limits::min(limits::max(0.0,self.x+mult[0]*normal(MEAN_MOVE,STD_MOVE,MAX_MOVE)),GRIDSIZE[self.zone as usize][0]);
                new_y = limits::min(limits::max(0.0,self.y+mult[1]*normal(MEAN_MOVE,STD_MOVE,MAX_MOVE)),GRIDSIZE[self.zone as usize][1]);        
                if FLY{
                    new_z = limits::min(limits::max(0.0,self.z+mult[2]*normal(MEAN_MOVE_Z,STD_MOVE_Z,MAX_MOVE_Z)),GRIDSIZE[self.zone as usize][2]);
                }
            }            
            host{infected:self.infected,motile:self.motile,zone:self.zone,prob1:self.prob1,prob2:self.prob2,x:new_x,y:new_y,z:self.z,age:self.age+1.0/HOUR_STEP,time:self.time+1.0/HOUR_STEP,origin_x:self.origin_x,origin_y:self.origin_y,origin_z:self.origin_z,restrict:self.restrict,range_x:self.range_x,range_y:self.range_y,range_z:self.range_z}
        }else if self.motile==0 && EVISCERATE_ZONES.contains(&self.zone){
            // println!("Evisceration pending...");
            // self.motile == 1; //It should be presumably electrocuted and hung on a conveyer belt
            self.x = ((self.origin_x as f64) + (self.range_x as f64))/2.0; // square in middle
            self.y = ((self.origin_y as f64) + (self.range_y as f64))/2.0;
            self.z = (self.origin_z as f64) + (self.range_z as f64); //Place chicken on the top of the box to simulate suspension on the top
            self.age += 1.0/HOUR_STEP;
            self.time += 1.0/HOUR_STEP;
            self
        }
        else if self.restrict{
            //deposits by hosts do not move obviously, but they DO age, which affects collection
            self.age += 1.0/HOUR_STEP;
            self.time += 1.0/HOUR_STEP;
            if FAECAL_DROP && self.motile == 2 && self.z>0.0{
                // println!("Examining poop for shuttle drop!");
                self.z -= (poisson(PROBABILITY_OF_FAECAL_DROP/HOUR_STEP)*(STEP[self.zone][2] as u64)) as f64;
                self.z = limits::max(self.z,0.0);
            }
            self
        }
        else{
            if self.z!=0.0{self.z = 0.0;}
            self.age+= 1.0/HOUR_STEP;
            self.time+=1.0/HOUR_STEP;
            self
        }
    }
    fn shuffle_all(vector: Vec<host>)->Vec<host>{
        vector.into_par_iter().map(|x| x.shuffle()).collect()
    }
    fn dist(host1: &host, host2: &host)->bool{
        let diff_x: f64 = host1.x -host2.x;
        let diff_y: f64 = host1.y - host2.y;
        let diff_z: f64 = host1.z - host2.z;
        let t: f64 = diff_x.powf(2.0)+diff_y.powf(2.0) + diff_z.powf(2.0);
        /////
        //PRINT STATEMENT
        // if t.powf(0.5)<=TRANSFER_DISTANCE{
        //     println!("{} {} vs {} {}",&host1.x,&host1.y,&host2.x,&host2.y);
        // }
        ////
        t.powf(0.5)<=TRANSFER_DISTANCE && host1.zone == host2.zone
    }
    // fn transmit(mut inventory:Vec<host>,time:usize)->Vec<host>{//Current version logic: Once the diseased host passes the "test" in fn transfer, then ALL other hosts within distance contract
    //     //Locate all infected hosts
    //     let mut cloneof: Vec<host> = inventory.clone();
    //     cloneof = cloneof.into_iter().filter_map(|mut x|{
    //         if x.infected{ //x.transfer is how we internalise the probabilistic nature (not definitive way) that a disease can or cannot spread from an infected individual
    //             Some(x)
    //         }else{
    //             None
    //         }
    //     }).collect();
    //     inventory = inventory.into_iter().filter(|x| !x.infected).collect::<Vec<host>>();    
    //     inventory = inventory.into_iter().filter_map(|mut x|{
    //         if cloneof.iter().any(|inf| host::dist(&inf,&x) && inf.zone == x.zone){
    //             let before = x.infected.clone();
    //             x.infected=x.transfer();
    //             if !before && x.infected{
    //                 if x.x!=0.0 && x.y != 0.0{println!("{} {} {} {} {}",x.x,x.y,x.z,time,x.zone);}
    //             }
    //             // println!("{} vs {}",&inf.x,&x.x,&inf.y,&x.y);
    //             Some(x)
    //         }else{
    //             Some(x)
    //         }
    //     }).collect();
    //     inventory.extend(cloneof);
    //     inventory
    // }
    fn transmit(mut inventory: Vec<host>, time: usize) -> Vec<host> {
        // Locate all infected hosts
        let mut cloneof: Vec<host> = inventory.clone();
        cloneof = cloneof
            .into_par_iter()
            .filter_map(|mut x| {
                if x.infected {
                    Some(x)
                } else {
                    None
                }
            })
            .collect();
        inventory = inventory.into_par_iter().filter(|x| !x.infected).collect::<Vec<host>>();
        inventory = inventory
            .into_par_iter()
            .filter_map(|mut x| {
                for inf in &cloneof {
                    if host::dist(inf, &x) && inf.zone == x.zone && (!TRANSFERS_ONLY_WITHIN || TRANSFERS_ONLY_WITHIN && x.origin_x == inf.origin_x && x.origin_y == inf.origin_y && x.origin_z == inf.origin_z){
                        let before = x.infected.clone();
                        x.infected = x.transfer();
                        if !before && x.infected {
                            if x.x != 0.0 && x.y != 0.0 {
                                let mut diagnostic:i8 = 1;
                                if x.motile>inf.motile{
                                    diagnostic = -1;
                                }
                                // Access properties of 'inf' here
                                println!(
                                    "{} {} {} {} {} {}",
                                    x.x,
                                    x.y,
                                    x.z,
                                    diagnostic*((x.motile+1) as i8) * ((inf.motile+1) as i8), // Access 'inf' properties here
                                    time,
                                    x.zone
                                );
                            }
                        }
                    }
                }
                Some(x)
            })
            .collect();
        inventory.extend(cloneof);
        inventory
    }
    
    fn cleanup(inventory:Vec<host>)->Vec<host>{
        inventory.into_par_iter().filter_map(|mut x|{
            if x.motile==2 || (!COLLECT_DEPOSITS && x.motile == 1){ // If host consumable deposits are not desired, treat them as equivalent to faeces to clean up
                // println!("Cleaning!");
                None
            }else{
                Some(x)
            }
        }).collect()
    }
    fn collect(inventory:Vec<host>)->[Vec<host>;2]{   //hosts and deposits potentially get collected
        let mut collection:Vec<host> = Vec::new();
        let vec1:Vec<host> = inventory.into_iter().filter_map(|mut x| {
            // println!("Chicken in zone {}",x.zone);
            if x.motile==0 && x.age>AGE_OF_HOSTCOLLECTION && x.zone == GRIDSIZE.len()-1{
                // println!("Collecting host(s)...{} days old",x.age/24.0);
                collection.push(x);
                None
            }else if x.motile == 1 && x.age>AGE_OF_DEPOSITCOLLECTION{
                // println!("Collecting deposit(s)...");
                collection.push(x);
                None
            }else{
                Some(x)
            }
        }).collect();
        [vec1,collection]  //collection vector here to be added and pushed into the original collection vector from the start of the loop! This function merely outputs what should be ADDED to collection!
    }
    fn collect__(inventory:Vec<host>,zone:&mut Zone_3D)->[Vec<host>;2]{   //hosts and deposits potentially get collected
        let mut collection:Vec<host> = Vec::new();
        let vec1:Vec<host> = inventory.into_iter().filter_map(|mut x| {
            // println!("Chicken in zone {}",x.zone);
            // println!("GRIDSIZE - 1 is {} ",GRIDSIZE.len()-1);
            if x.motile==0 && x.time>TIME_OF_COLLECTION && x.zone == GRIDSIZE.len()-1{
                // println!("Collecting host(s)...{} days old",x.age/24.0);
                zone.subtract(x.origin_x,x.origin_y,x.origin_z);
                collection.push(x);
                // *capacity+=1;
                None
            }else if x.motile == 1 && x.age>AGE_OF_DEPOSITCOLLECTION && COLLECT_DEPOSITS{
                // println!("Collecting deposit(s)...");
                collection.push(x);
                // *capacity+=1;
                None
            }else{
                Some(x)
            }
        }).collect();
        [vec1,collection]  //collection vector here to be added and pushed into the original collection vector from the start of the loop! This function merely outputs what should be ADDED to collection!
    }    
    fn collect_and_replace(inventory:Vec<host>)->[Vec<host>;2]{   //same as collect but HOSTS GET REPLACED (with a Poisson rate of choosing) - note that this imports hosts, doesn't transfer from earlier zone
        let mut collection:Vec<host> = Vec::new();
        let vec1:Vec<host> = inventory.into_iter().filter_map(|mut x| {
            if x.motile==0 && x.age>AGE_OF_HOSTCOLLECTION&& x.zone == GRIDSIZE.len()-1{
                // println!("Collecting host(s)...{} days old",x.age/24.0);
                collection.push(x.clone());
                // None
                let mut rng = thread_rng();
                let roll = Uniform::new(0.0, 1.0);
                let rollnumber: f64 = rng.sample(roll);
                if rollnumber<PROBABILITY_OF_INFECTION{
                    Some(host{infected:true,age:normal(MEAN_AGE,STD_AGE,MAX_AGE),time:0.0,..x})
                }else{
                    Some(host{infected:false,age:normal(MEAN_AGE,STD_AGE,MAX_AGE),time:0.0,..x})
                }
            }else if x.motile == 1 && x.age>AGE_OF_DEPOSITCOLLECTION{
                // println!("Collecting deposit(s)...");
                collection.push(x);
                None
            }else{
                Some(x)
            }
        }).collect();
        [vec1,collection]  //collection vector here to be added and pushed into the original collection vector from the start of the loop! This function merely outputs what should be ADDED to collection!
    }
    fn report(inventory:&Vec<host>)->[f64;4]{ //simple function to quickly return the percentage of infected hosts
        let inf: f64 = inventory.clone().into_iter().filter(|x| {
            x.infected && x.motile==0
        }).collect::<Vec<_>>().len() as f64;
        let noofhosts: f64 = inventory.clone().into_iter().filter(|x| {
            x.motile==0
        }).collect::<Vec<_>>().len() as f64;

        let inf2: f64 = inventory.clone().into_iter().filter(|x| {
            x.infected && x.motile==1
        }).collect::<Vec<_>>().len() as f64;
        let noofhosts2: f64 = inventory.clone().into_iter().filter(|x| {
            x.motile==1
        }).collect::<Vec<_>>().len() as f64;        

        [inf/(noofhosts+1.0),inf2/(noofhosts2+1.0),noofhosts,noofhosts2]
    }
    fn zone_report(inventory:&Vec<host>,zone:usize)->[f64;4]{ //simple function to quickly return the percentage of infected hosts
        let mut inventory:Vec<host> = inventory.clone().into_iter().filter(|x|{
            x.zone == zone
        }).collect::<Vec<_>>();
        let inf: f64 = inventory.clone().into_iter().filter(|x| {
            x.infected && x.motile==0
        }).collect::<Vec<_>>().len() as f64;
        let noofhosts: f64 = inventory.clone().into_iter().filter(|x| {
            x.motile==0
        }).collect::<Vec<_>>().len() as f64;

        let inf2: f64 = inventory.clone().into_iter().filter(|x| {
            x.infected && x.motile==1
        }).collect::<Vec<_>>().len() as f64;
        let noofhosts2: f64 = inventory.clone().into_iter().filter(|x| {
            x.motile==1
        }).collect::<Vec<_>>().len() as f64;        

        [inf/(noofhosts+1.0),inf2/(noofhosts2+1.0),noofhosts,noofhosts2]
    }    
    fn generate_in_grid(zone:&mut Zone_3D,hosts:&mut Vec<host>){  //Fill up each segment completely to full capacity in a zone with chickens. Also update the capacity to reflect that there is no more space
        let zone_no:usize = zone.clone().zone;
        zone.segments.iter_mut().for_each(|mut x| {
            let mean_x:f64 = ((x.range_x as f64)/2.0) as f64;
            let std_x:f64 = ((x.range_x as f64)/SPORADICITY) as f64;
            let max_x:f64 = x.range_x as f64;
            let mean_y:f64 = ((x.range_y as f64)/2.0) as f64;
            let std_y:f64 = ((x.range_y as f64)/SPORADICITY) as f64;
            let max_y:f64 = x.range_y as f64;            
            for _ in 0..x.capacity.clone() as usize{hosts.push(host::new(zone_no,0.2,x.origin_x as f64 + normal(mean_x,std_x,max_x),(x.origin_y as f64 +normal(mean_y,std_y,max_y)) as f64,x.origin_z as f64,RESTRICTION,x.range_x,x.range_y,x.range_z));}
            x.capacity = 0;
            zone.capacity = 0;
        });
    }
}


fn main(){
    let mut chickens: Vec<host> = Vec::new();
    // let mut feast: Vec<host> =  Vec::new();
    let mut hosts_in_collection:[u64;2] = [0,1];
    let mut deposits_in_collection:[u64;2] = [0,1];
    let mut zones:Vec<Zone_3D> = Vec::new();

    //Influx parameter
    let mut influx:bool = false;
    //Generate eviscerators
    let mut eviscerators:Vec<Eviscerator> = Vec::new();
    if EVISCERATE{
        for index in 0..EVISCERATE_ZONES.len(){
            for _ in 0..NO_OF_EVISCERATORS[index]{
                eviscerators.push(Eviscerator{zone:EVISCERATE_ZONES[index],infected:false,count_since_infected:0})
            }
        }
    }
    
    //Initialise with chickens in the first zone only
    for grid in 0..GRIDSIZE.len(){
        zones.push(Zone_3D::generate_empty(grid,[GRIDSIZE[grid][0] as u64,GRIDSIZE[grid][1] as u64,GRIDSIZE[grid][2] as u64],STEP[grid]));
    }

    host::generate_in_grid(&mut zones[0],&mut chickens);
    // println!("{:?}", chickens.len());
    // for thing in chickens.clone(){
    //     println!("Located at zone {} in {} {}: MOTION PARAMS: {} for {} and {} for {}",thing.zone,thing.x,thing.y,thing.origin_x,thing.range_x,thing.origin_y,thing.range_y);
    // }
    //GENERATE INFECTED HOST
    // chickens.push(host::new_inf(1,0.2,(GRIDSIZE[0] as u64)/2,(GRIDSIZE[1] as u64)/2),true,STEP as u64,STEP as u64); // the infected
    // chickens = host::infect(chickens,400,400,0);
    // chickens = host::infect(chickens,800,800,0);
    // chickens = host::infect(chickens,130,40,0);
    // chickens = host::infect(chickens,10,10,0);
    // chickens = host::infect(chickens,300,1800,0);

    //MORE EFFICIENT WAY TO INFECT MORE CHICKENS - insize zone 0
    let zone_to_infect:usize = 0;
    chickens = host::infect_multiple(chickens,GRIDSIZE[zone_to_infect][0] as u64/2,GRIDSIZE[zone_to_infect][1] as u64/2,GRIDSIZE[zone_to_infect][2] as u64/2,2,0);


    //Count number of infected
    // let it: u64 = chickens.clone().into_iter().filter(|x| x.infected).collect()::<Vec<_>>.len();
    // let mut vecc_into: Vec<host> = chickens.clone().into_iter().filter(|x| x.infected).collect::<Vec<_>>(); //With this re are RETAINING the hosts and deposits within the original vector
    // println!("NUMBER OF INFECTED CHICKENS IS {}", vecc_into.len());
    //CSV FILE
    let filestring: String = format!("./output.csv");
    if fs::metadata(&filestring).is_ok() {
        fs::remove_file(&filestring).unwrap();
    }
    // Open the file in append mode for writing
    let mut file = OpenOptions::new()
    .write(true)
    .create(true)
    .append(true) // Open in append mode
    .open(&filestring)
    .unwrap();
    let mut wtr = Writer::from_writer(file);
    for time in 0..LENGTH{
        let mut collect: Vec<host> = Vec::new();
        if time % (24/FAECAL_CLEANUP_FREQUENCY) ==0{
            chickens = host::cleanup(chickens);
        }
        // println!("{} CHECK {}",time%(PERIOD_OF_TRANSPORT  as usize),time%(PERIOD_OF_TRANSPORT  as usize) == 0);
        if time%(PERIOD_OF_TRANSPORT  as usize)==0{
            // println!("Fulfilling period of transport right now");
            host::transport(&mut chickens,&mut zones,influx);
            // println!("Total number of chickens is {}: Total number of faeces is {}",  chickens.clone().into_iter().filter(|x| x.motile == 0).collect::<Vec<_>>().len() as u64,chickens.clone().into_iter().filter(|x| x.motile == 2).collect::<Vec<_>>().len() as u64)
        }        
        for times in FEED_TIMES{
            if time % times == 0 && time>0 && FEED{
                for spaces in FEED_ZONES{
                    chickens = zones[spaces].clone().feed_setup(chickens,time.clone());
                }
            }
        }
        if EVISCERATE{
            for zone in EVISCERATE_ZONES{
                // println!("Evisceration occurring at zone {}",zone);
                zones[zone].eviscerate(&mut eviscerators,&mut chickens,time.clone());
            }
        }
        let mut collection_counter_fromFinalZone:&mut Zone_3D = &mut zones[GRIDSIZE.len()-1];
        [chickens,collect] = host::collect__(chickens,&mut collection_counter_fromFinalZone);

        for unit in 0..HOUR_STEP as usize{
            // println!("Number of poop is {}",chickens.clone().into_iter().filter(|x| x.motile == 2).collect::<Vec<_>>().len() as u64);
            chickens = host::shuffle_all(chickens);
            chickens = host::transmit(chickens,time.clone());
            if FLY && unit != 0 && (unit % FLY_FREQ as usize) == 0{
                chickens = host::land(chickens);
            }
        } //Say chickens move/don't move every 15min - 4 times per hour
        chickens = host::deposit_all(chickens);
        //Collect the hosts and deposits as according
        // println!("Number of infected eggs in soon to be collection is {}",collect.clone().into_iter().filter(|x| x.motile == 1 && x.infected).collect::<Vec<_>>().len() as f64);
        // feast.append(&mut collect);
        //Update Collection numbers
        let no_of_infected_hosts: u64 = collect.clone().into_par_iter().filter(|x| x.motile == 0 && x.infected).collect::<Vec<_>>().len() as u64;
        let no_of_hosts: u64 = collect.clone().into_par_iter().filter(|x| x.motile == 0).collect::<Vec<_>>().len() as u64;
        let no_of_deposits: u64 = collect.clone().into_par_iter().filter(|x| x.motile == 1).collect::<Vec<_>>().len() as u64;
        let no_of_infected_deposits: u64 = collect.clone().into_par_iter().filter(|x| x.motile == 1 && x.infected).collect::<Vec<_>>().len() as u64;

        hosts_in_collection[0] += no_of_infected_hosts;
        hosts_in_collection[1] += no_of_hosts;
        deposits_in_collection[0] += no_of_infected_deposits;
        deposits_in_collection[1] += no_of_deposits;

        if INFLUX && time%PERIOD_OF_INFLUX as usize==0 && time>0{
            influx = true;
            // println!("Influx just got changed to true");
        }else{
            influx = false;
        }
        // let mut count:u8 = 0;
        // for i in zones.clone(){
        //     println!("{} : {}", count,i.capacity);
        //     count+=1;
        // }
        //Farm
        let no_of_zones:usize = GRIDSIZE.len();
        let collection_zone_no:u8 = no_of_zones as u8+1;
        //Call once
        for iter in 0..no_of_zones{
            let [mut perc,mut perc2,mut total_hosts,mut total_hosts2] = host::zone_report(&chickens,iter);            
            let no = perc.clone()*total_hosts;
            perc = perc*100.0;
            let no2 = perc2.clone()*total_hosts2;        
            perc2 = perc2*100.0;
            wtr.write_record(&[
                perc.to_string(),
                total_hosts.to_string(),
                no.to_string(),
                perc2.to_string(),
                total_hosts2.to_string(),
                no2.to_string(),
                format!("Zone {}", iter),
            ]);
        }

        //Collection
        // let [mut _perc,mut _perc2,mut _total_hosts,mut _total_hosts2] = host::report(&feast);
        let _no = hosts_in_collection[0];
        let _perc = (hosts_in_collection[0] as f64)/(hosts_in_collection[1] as f64) * 100.0;
        let _no2 = deposits_in_collection[0];
        let _perc2 = (deposits_in_collection[0] as f64)/(deposits_in_collection[1] as f64)*100.0;
        let _total_hosts = hosts_in_collection[1];
        let _total_hosts2 = deposits_in_collection[1];
        // println!("{} {} {} {} {} {}",perc,total_hosts,no,perc2,total_hosts2,no2);    
        // println!("{} {} {} {} {} {} {} {} {} {} {} {}",perc,total_hosts,no,perc2,total_hosts2,no2,_perc,_total_hosts,_no,_perc2,_total_hosts2,_no2);
        wtr.write_record(&[
            _perc.to_string(),
            _total_hosts.to_string(),
            _no.to_string(),
            _perc2.to_string(),
            _total_hosts2.to_string(),
            _no2.to_string(),
            "Collection Zone".to_string(),
        ])
        .unwrap();

        // if host::report(&chickens)[2]<5.0{break;}
    }
    wtr.flush().unwrap();
    // println!("{} {} {} {} {} {}",STEP[0][0],STEP[0][1],STEP[0][2],LENGTH,GRIDSIZE.len(), TRANSFER_DISTANCE); //Last 5 lines are going to be zone config lines that need to be picked out in plotter.py
    for zone in 0..GRIDSIZE.len(){
        println!("{} {} {} {} {} {}",GRIDSIZE[zone][0],GRIDSIZE[zone][1],GRIDSIZE[zone][2],1000,0,zone);
    }
    for zone in 0..GRIDSIZE.len(){
        // println!("{} {} {} {} {} {}",GRIDSIZE[zone][0],GRIDSIZE[zone][1],GRIDSIZE[zone][2],1000,0,zone);
        println!("{} {} {} {} {} {}",STEP[zone][0]+100000,STEP[zone][1]+100000,STEP[zone][2]+100000,GRIDSIZE[zone][0]+100000.0,GRIDSIZE[zone][1]+100000.0,GRIDSIZE[zone][2]+100000.0);  //Paramters for R file to extract and plot
    }
    // println!("{} {} {} {} {} {}",GRIDSIZE[0][0],GRIDSIZE[0][1],GRIDSIZE[0][2],0,0,0); //Last 5 lines are going to be zone config lines that need to be picked out in plotter.py
    
    // Open a file for writing
    let mut file = File::create("parameters.txt").expect("Unable to create file");


    // Write constants to the file
    // Space
    writeln!(file, "## Space").expect("Failed to write to file");
    writeln!(file, "- RESTRICTION: {} (Are the hosts restricted to segments within each zone)", RESTRICTION).expect("Failed to write to file");
    writeln!(file, "- LISTOFPROBABILITIES: {:?} (Probability of transfer of salmonella per zone)", LISTOFPROBABILITIES).expect("Failed to write to file");
    writeln!(file, "- GRIDSIZE: {:?} (Size of the grid)", GRIDSIZE).expect("Failed to write to file");
    writeln!(file, "- MAX_MOVE: {} (Maximum move value)", MAX_MOVE).expect("Failed to write to file");
    writeln!(file, "- MEAN_MOVE: {} (Mean move value)", MEAN_MOVE).expect("Failed to write to file");
    writeln!(file, "- STD_MOVE: {} (Standard deviation of move value)", STD_MOVE).expect("Failed to write to file");
    writeln!(file, "- MAX_MOVE_Z: {} (Maximum move for vertical motion)", MAX_MOVE_Z).expect("Failed to write to file");
    writeln!(file, "- MEAN_MOVE_Z: {} (Mean move value for vertical motion)", MEAN_MOVE_Z).expect("Failed to write to file");
    writeln!(file, "- STD_MOVE_Z: {} (Standard deviation of move value for vertical motion)", STD_MOVE_Z).expect("Failed to write to file");
    writeln!(file, "- FAECAL DROP: {} (Does faeces potentially fall between segments downwards?)", FAECAL_DROP).expect("Failed to write to file");
    writeln!(file, "- PROBABILITY OF FAECAL DROP: {} (If yes, what is the probability? -> Poisson hourly rate)", PROBABILITY_OF_FAECAL_DROP).expect("Failed to write to file");
    writeln!(file, "- TRANSMISSION BETWEEN ZONES enabled: {} (Can diseases transfer between segments/cages within each zone?)", !TRANSFERS_ONLY_WITHIN).expect("Failed to write to file");
    // Fly configuration
    writeln!(file, "\n## Flight module").expect("Failed to write to file");
    writeln!(file, "- FLY: {} (Flight module enabled/disabled)", FLY).expect("Failed to write to file");    
    writeln!(file, "- FLY_FREQ: {} (Frequency of flight - which HOUR STEP do the hosts land, if at all)", FLY_FREQ).expect("Failed to write to file");    

    // Eviscerator configuration
    writeln!(file, "\n## Eviscerator Configuration enabled:{}",EVISCERATE).expect("Failed to write to file");
    writeln!(file, "- Evisceration Zones: {:?}", EVISCERATE_ZONES).expect("Failed to write to file");   
    writeln!(file, "- NUMBER OF EVISCERATORS: {:?}", NO_OF_EVISCERATORS).expect("Failed to write to file");   
    writeln!(file, "- EVISCERATOR DECAY: {} (Number of hosts an eviscerator has to go through before the infection is gone)", EVISCERATE_DECAY).expect("Failed to write to file");        

    // Transfer config
    writeln!(file, "\n## Transfer Configuration").expect("Failed to write to file");
    writeln!(file, "- Times Manual Map: {:?} (Times that hosts have to spend in each zone)", ages).expect("Failed to write to file");    
    writeln!(file, "- Influx?: {} (Did the simulation bring in chickens to process?)", INFLUX).expect("Failed to write to file");     
    writeln!(file, "- If yes, they were brought in every {} hours", PERIOD_OF_INFLUX).expect("Failed to write to file");        
    writeln!(file, "- Period of transport rules : {} hours (How many hours until we check to see if hosts need to be moved from zone to zone)", PERIOD_OF_TRANSPORT).expect("Failed to write to file");        

    // Disease
    writeln!(file, "\n## Disease").expect("Failed to write to file");
    writeln!(file, "- TRANSFER_DISTANCE: {} (Maximum distance for disease transmission)", TRANSFER_DISTANCE).expect("Failed to write to file");

    // Collection
    writeln!(file, "\n## Collection").expect("Failed to write to file");
    writeln!(file, "- AGE_OF_HOSTCOLLECTION: {} days", AGE_OF_HOSTCOLLECTION/24.0).expect("Failed to write to file");
    writeln!(file, "- AGE_OF_DEPOSITCOLLECTION: {} days", AGE_OF_DEPOSITCOLLECTION/24.0).expect("Failed to write to file");
    writeln!(file, "- FAECAL_CLEANUP_FREQUENCY: {} times per day", 24/FAECAL_CLEANUP_FREQUENCY).expect("Failed to write to file");

    // Resolution
    writeln!(file, "\n## Resolution").expect("Failed to write to file");
    writeln!(file, "- STEP: {:?} (Chickens per unit distance)", STEP).expect("Failed to write to file");
    writeln!(file, "- HOUR_STEP: {} (Chickens move per hour)", HOUR_STEP).expect("Failed to write to file");
    writeln!(file, "- LENGTH: {} (Simulation duration in hours)", LENGTH).expect("Failed to write to file");

    //Generation
    writeln!(file, "\n## Generation").expect("Failed to write to file");
    writeln!(file, "- SPORADICITY: {} ( Bigger number makes the spread of hosts starting point more even per seg)", SPORADICITY).expect("Failed to write to file");


}
